use anyhow::Context;
use diesel::{PgConnection, RunQueryDsl};
use orm::balances::BalanceChangesInsertDb;
use orm::ibc::IbcRateLimitsInsertDb;
use orm::schema::{
    balance_changes, ibc_rate_limits, ibc_token, token,
    token_supplies_per_epoch,
};
use orm::token::{IbcTokenInsertDb, TokenInsertDb};
use orm::token_supplies_per_epoch::TokenSuppliesInsertDb;
use shared::balance::{Balances, TokenSupply};
use shared::token::{IbcRateLimit, Token};
use shared::tuple_len::TupleLen;

use super::utils::MAX_PARAM_SIZE;

pub fn insert_balances(
    transaction_conn: &mut PgConnection,
    balances: Balances,
) -> anyhow::Result<()> {
    let balances_col_count = balance_changes::all_columns.len() as i64;

    for chunk in
        balances.chunks((MAX_PARAM_SIZE as i64 / balances_col_count) as usize)
    {
        diesel::insert_into(balance_changes::table)
            .values::<&Vec<BalanceChangesInsertDb>>(
                &chunk
                    .iter()
                    .cloned()
                    .map(BalanceChangesInsertDb::from_balance)
                    .collect::<Vec<_>>(),
            )
            .on_conflict((
                balance_changes::columns::owner,
                balance_changes::columns::token,
                balance_changes::columns::height,
            ))
            .do_nothing()
            .execute(transaction_conn)
            .context("Failed to update balances in db")?;
    }

    anyhow::Ok(())
}

pub fn insert_tokens(
    transaction_conn: &mut PgConnection,
    tokens: Vec<Token>,
) -> anyhow::Result<()> {
    let tokens_db = tokens.iter().map(TokenInsertDb::from).collect::<Vec<_>>();

    let ibc_tokens_db = tokens
        .iter()
        .filter_map(IbcTokenInsertDb::from_token)
        .collect::<Vec<_>>();

    // TODO: add tests for on conflict:
    // - inserting token with the same address twice should NOT throw an error
    // - inserting native token with different address should throw an error
    diesel::insert_into(token::table)
        .values(tokens_db)
        .on_conflict(token::columns::address)
        .do_nothing()
        .execute(transaction_conn)
        .context("Failed to update tokens in db")?;

    diesel::insert_into(ibc_token::table)
        .values(ibc_tokens_db)
        .on_conflict(ibc_token::columns::address)
        .do_nothing()
        .execute(transaction_conn)
        .context("Failed to update ibc tokens in db")?;

    anyhow::Ok(())
}

pub fn insert_token_supplies<S>(
    transaction_conn: &mut PgConnection,
    supplies: S,
) -> anyhow::Result<()>
where
    S: IntoIterator<Item = TokenSupply>,
{
    let supplies: Vec<_> = supplies
        .into_iter()
        .map(TokenSuppliesInsertDb::from)
        .collect();

    if supplies.is_empty() {
        return anyhow::Ok(());
    }

    tracing::debug!(?supplies, "Adding new token supplies to db");

    diesel::insert_into(token_supplies_per_epoch::table)
        .values(supplies)
        .on_conflict_do_nothing()
        .execute(transaction_conn)
        .context("Failed to update token supplies in db")?;

    anyhow::Ok(())
}

pub fn insert_ibc_rate_limits<S>(
    transaction_conn: &mut PgConnection,
    supplies: S,
) -> anyhow::Result<()>
where
    S: IntoIterator<Item = IbcRateLimit>,
{
    let limits: Vec<_> = supplies
        .into_iter()
        .map(IbcRateLimitsInsertDb::from)
        .collect();

    if limits.is_empty() {
        return anyhow::Ok(());
    }

    diesel::insert_into(ibc_rate_limits::table)
        .values(limits)
        .on_conflict_do_nothing()
        .execute(transaction_conn)
        .context("Failed to update rate limits in db")?;

    anyhow::Ok(())
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use anyhow::Context;
    use diesel::{
        BoolExpressionMethods, ExpressionMethods, QueryDsl, SelectableHelper,
    };
    use namada_sdk::token::Amount as NamadaAmount;
    use namada_sdk::uint::MAX_SIGNED_VALUE;
    use orm::balances::BalanceDb;
    use orm::blocks::BlockInsertDb;
    use orm::schema::blocks;
    use orm::views::balances;
    use shared::balance::{Amount, Balance};
    use shared::id::Id;
    use shared::token::IbcToken;
    use test_helpers::db::TestDb;

    use super::*;

    /// Test that the function correctly handles an empty `balances` input.
    #[tokio::test]
    async fn test_insert_balance_with_empty_balances() {
        let db = TestDb::new();

        db.run_test(|conn| {
            insert_balances(conn, vec![])?;

            let queried_balance = query_all_balances(conn)?;

            assert_eq!(queried_balance.len(), 0,);

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test the basic functionality of inserting a single balance.
    #[tokio::test]
    async fn test_insert_balance_with_single_balance() {
        let db = TestDb::new();

        db.run_test(|conn| {
            let owner = Id::Account(
                "tnam1qqshvryx9pngpk7mmzpzkjkm6klelgusuvmkc0uz".to_string(),
            );
            let token = Token::Native(Id::Account(
                "tnam1q87wtaqqtlwkw927gaff34hgda36huk0kgry692a".to_string(),
            ));
            let amount = Amount::from(NamadaAmount::from_u64(100));
            let height = 42;

            let balance = Balance {
                owner: owner.clone(),
                token: token.clone(),
                amount: amount.clone(),
                height,
            };

            insert_tokens(conn, vec![token.clone()])?;

            seed_blocks_from_balances(conn, &[balance.clone()])?;

            insert_balances(conn, vec![balance.clone()])?;

            let queried_balance = query_balance_by_address(conn, owner, token)?;

            assert_eq!(Amount::from(queried_balance.raw_amount), amount);

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test that the function updates existing balances when there is a
    /// later height.
    #[tokio::test]
    async fn test_insert_balance_with_existing_balances_update() {
        let db = TestDb::new();

        let owner = Id::Account(
            "tnam1qqshvryx9pngpk7mmzpzkjkm6klelgusuvmkc0uz".to_string(),
        );
        let token = Token::Native(Id::Account(
            "tnam1q87wtaqqtlwkw927gaff34hgda36huk0kgry692a".to_string(),
        ));
        let amount = Amount::from(NamadaAmount::from_u64(100));
        let height = 42;

        let balance = Balance {
            owner: owner.clone(),
            token: token.clone(),
            amount: amount.clone(),
            height,
        };

        db.run_test(move |conn| {
            seed_balance(conn, vec![balance.clone()])?;

            let new_amount = Amount::from(NamadaAmount::from_u64(200));
            let new_height = 43;
            let new_balance = Balance {
                amount: new_amount.clone(),
                height: new_height,
                ..(balance.clone())
            };

            seed_blocks_from_balances(conn, &[new_balance.clone()])?;
            insert_balances(conn, vec![new_balance])?;

            let queried_balance =
                query_balance_by_address(conn, owner.clone(), token.clone())?;

            assert_eq!(Amount::from(queried_balance.raw_amount), new_amount);

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test the function's behavior when inserting balances that cause a
    /// conflict (same owner, different token).
    #[tokio::test]
    async fn test_insert_balance_with_conflicting_owners() {
        let db = TestDb::new();

        let owner = Id::Account(
            "tnam1qqshvryx9pngpk7mmzpzkjkm6klelgusuvmkc0uz".to_string(),
        );
        let token = Token::Native(Id::Account(
            "tnam1qxfj3sf6a0meahdu9t6znp05g8zx4dkjtgyn9gfu".to_string(),
        ));
        let amount = Amount::from(NamadaAmount::from_u64(100));
        let height = 42;

        let balance = Balance {
            owner: owner.clone(),
            token: token.clone(),
            amount: amount.clone(),
            height,
        };

        db.run_test(move |conn| {
            seed_balance(conn, vec![balance.clone()])?;

            // this is probably not a valid way to construct an IbcToken
            // but seems to be sufficient for testing purposes here.
            let new_token = Token::Ibc(IbcToken {
                address: Id::Account(
                    "tnam1q9rhgyv3ydq0zu3whnftvllqnvhvhm270qxay5tn".to_string(),
                ),
                trace: Some(Id::Account(
                    "tnam1q9rhgyv3ydq0zu3whnftvllqnvhvhm270qxay5tn".to_string(),
                )),
            });

            let new_amount = Amount::from(NamadaAmount::from_u64(200));
            let new_balance = Balance {
                token: new_token.clone(),
                amount: new_amount.clone(),
                ..(balance.clone())
            };

            seed_tokens_from_balance(conn, vec![new_balance.clone()])?;

            insert_balances(conn, vec![new_balance])?;

            let queried_balance =
                query_balance_by_address(conn, owner.clone(), token.clone())?;

            let queried_balance_new = query_balance_by_address(
                conn,
                owner.clone(),
                new_token.clone(),
            )?;

            assert_eq!(Amount::from(queried_balance.raw_amount), amount);
            assert_eq!(
                Amount::from(queried_balance_new.raw_amount),
                new_amount
            );

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }
    /// Test the function's behavior when inserting balances that cause a
    /// conflict. (same token, different owner)
    #[tokio::test]
    async fn test_insert_balance_with_conflicting_tokens() {
        let db = TestDb::new();

        let owner = Id::Account(
            "tnam1qqshvryx9pngpk7mmzpzkjkm6klelgusuvmkc0uz".to_string(),
        );
        let token = Token::Native(Id::Account(
            "tnam1qxfj3sf6a0meahdu9t6znp05g8zx4dkjtgyn9gfu".to_string(),
        ));
        let amount = Amount::from(NamadaAmount::from_u64(100));
        let height = 42;

        let balance = Balance {
            owner: owner.clone(),
            token: token.clone(),
            amount: amount.clone(),
            height,
        };

        db.run_test(move |conn| {
            seed_balance(conn, vec![balance.clone()])?;

            let new_owner = Id::Account(
                "tnam1q9rhgyv3ydq0zu3whnftvllqnvhvhm270qxay5tn".to_string(),
            );
            let new_amount = Amount::from(NamadaAmount::from_u64(200));
            let new_balance = Balance {
                amount: new_amount.clone(),
                owner: new_owner.clone(),
                ..(balance.clone())
            };

            insert_balances(conn, vec![new_balance])?;

            let queried_balance =
                query_balance_by_address(conn, owner.clone(), token.clone())?;

            let queried_balance_new = query_balance_by_address(
                conn,
                new_owner.clone(),
                token.clone(),
            )?;

            assert_eq!(Amount::from(queried_balance.raw_amount), amount);
            assert_eq!(
                Amount::from(queried_balance_new.raw_amount),
                new_amount
            );

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test the function's behavior when inserting balances that cause a
    /// conflict (same owner, same token, same height, different amount).
    #[tokio::test]
    async fn test_insert_balance_with_conflicting_heights() {
        let db = TestDb::new();

        let owner = Id::Account(
            "tnam1qqshvryx9pngpk7mmzpzkjkm6klelgusuvmkc0uz".to_string(),
        );
        let token = Token::Native(Id::Account(
            "tnam1qxfj3sf6a0meahdu9t6znp05g8zx4dkjtgyn9gfu".to_string(),
        ));
        let amount = Amount::from(NamadaAmount::from_u64(100));
        let height = 42;

        let balance = Balance {
            owner: owner.clone(),
            token: token.clone(),
            amount: amount.clone(),
            height,
        };

        db.run_test(move |conn| {
            seed_balance(conn, vec![balance.clone()])?;

            let new_amount = Amount::from(NamadaAmount::from_u64(200));
            let new_balance = Balance {
                amount: new_amount.clone(),
                ..(balance.clone())
            };

            let res = insert_balances(conn, vec![new_balance]);

            // Conflicting insert succeeds, but is ignored
            assert!(res.is_ok());

            // Balance is not updated when height is the same
            let queried_balance =
                query_balance_by_address(conn, owner.clone(), token.clone())?;
            assert_eq!(Amount::from(queried_balance.raw_amount), amount);

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test the function's ability to handle a large number of balance inserts
    /// efficiently.
    #[tokio::test]
    async fn test_insert_balance_with_large_number_of_balances() {
        let db = TestDb::new();

        db.run_test(move |conn| {
            let token = Token::Native(Id::Account(
                "tnam1q87wtaqqtlwkw927gaff34hgda36huk0kgry692a".to_string(),
            ));
            let fake_balances = (0..10000)
                .map(|_| Balance::fake_with_token(token.clone()))
                .collect::<Vec<_>>();

            seed_tokens_from_balance(conn, fake_balances.clone())?;

            seed_blocks_from_balances(conn, &fake_balances)?;

            insert_balances(conn, fake_balances.clone())?;

            assert_eq!(query_all_balances(conn)?.len(), fake_balances.len());

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test how the function handles extremely large balance values.
    #[tokio::test]
    async fn test_insert_balance_with_extremely_large_balance_value() {
        let db = TestDb::new();

        db.run_test(|conn| {
            let owner = Id::Account(
                "tnam1qqshvryx9pngpk7mmzpzkjkm6klelgusuvmkc0uz".to_string(),
            );
            let token = Token::Native(Id::Account(
                "tnam1q87wtaqqtlwkw927gaff34hgda36huk0kgry692a".to_string(),
            ));
            let max_amount = Amount::from(NamadaAmount::from(MAX_SIGNED_VALUE));
            let height = 42;

            let balance = Balance {
                owner: owner.clone(),
                token: token.clone(),
                amount: max_amount.clone(),
                height,
            };

            insert_tokens(conn, vec![token.clone()])?;

            seed_blocks_from_balances(conn, &[balance.clone()])?;
            insert_balances(conn, vec![balance.clone()])?;

            let queried_balance = query_balance_by_address(conn, owner, token)?;

            assert_eq!(Amount::from(queried_balance.raw_amount), max_amount);

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test that we can insert more than u16::MAX balances
    #[tokio::test]
    async fn test_insert_balances_with_max_param_size_plus_one() {
        let db = TestDb::new();

        db.run_test(|conn| {
            let mps = MAX_PARAM_SIZE as u32;

            let token = Token::Native(Id::Account(
                "tnam1q87wtaqqtlwkw927gaff34hgda36huk0kgry692a".to_string(),
            ));

            // We have to fake_with_token otherwise we won't be able to seed
            // MAX_PARAM_SIZE + 1 tokens and test will panic
            let balances = (0..mps + 1)
                .map(|_| Balance::fake_with_token(token.clone()))
                .collect::<Vec<_>>();

            insert_tokens(conn, vec![token])?;

            seed_blocks_from_balances(conn, &balances)?;

            let res = insert_balances(conn, balances);

            assert!(res.is_ok());

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    /// Test that we can insert less than u16::MAX balances using chunks
    #[tokio::test]
    async fn test_insert_balances_with_1000_params() {
        let db = TestDb::new();

        db.run_test(|conn| {
            let token = Token::Native(Id::Account(
                "tnam1q87wtaqqtlwkw927gaff34hgda36huk0kgry692a".to_string(),
            ));
            let balances = (0..1000)
                .map(|_| Balance::fake_with_token(token.clone()))
                .collect::<Vec<_>>();

            insert_tokens(
                conn,
                balances
                    .iter()
                    .map(|balance| balance.token.clone())
                    .collect::<Vec<_>>(),
            )?;

            seed_tokens_from_balance(conn, balances.clone())?;

            seed_blocks_from_balances(conn, &balances)?;

            let res = insert_balances(conn, balances);

            assert!(res.is_ok());

            anyhow::Ok(())
        })
        .await
        .expect("Failed to run test");
    }

    fn seed_tokens_from_balance(
        conn: &mut PgConnection,
        balance: Vec<Balance>,
    ) -> anyhow::Result<()> {
        insert_tokens(
            conn,
            balance
                .iter()
                .map(|balance| balance.token.clone())
                .collect::<Vec<_>>(),
        )?;

        anyhow::Ok(())
    }

    fn seed_blocks_from_balances(
        conn: &mut PgConnection,
        balances: &[Balance],
    ) -> anyhow::Result<()> {
        for height in balances
            .iter()
            .map(|balance| balance.height as i32)
            .collect::<HashSet<_>>()
        {
            diesel::insert_into(blocks::table)
                .values::<&BlockInsertDb>(&BlockInsertDb::fake(height))
                .on_conflict_do_nothing()
                .execute(conn)
                .context("Failed to insert block in db")?;
        }

        anyhow::Ok(())
    }

    fn seed_balance(
        conn: &mut PgConnection,
        balances: Vec<Balance>,
    ) -> anyhow::Result<()> {
        seed_tokens_from_balance(conn, balances.clone())?;

        seed_blocks_from_balances(conn, &balances)?;

        diesel::insert_into(balance_changes::table)
            .values::<&Vec<BalanceChangesInsertDb>>(
                &balances
                    .into_iter()
                    .map(BalanceChangesInsertDb::from_balance)
                    .collect::<Vec<_>>(),
            )
            .execute(conn)
            .context("Failed to update balances in db")?;

        anyhow::Ok(())
    }

    fn query_balance_by_address(
        conn: &mut PgConnection,
        owner: Id,
        token: Token,
    ) -> anyhow::Result<BalanceDb> {
        let token = match token {
            Token::Native(token) => token.to_string(),
            Token::Ibc(token) => token.address.to_string(),
        };

        balances::table
            .filter(
                balances::dsl::owner
                    .eq(owner.to_string())
                    .and(balances::dsl::token.eq(token)),
            )
            .select(BalanceDb::as_select())
            .first(conn)
            .context("Failed to query balance by address")
    }

    fn query_all_balances(
        conn: &mut PgConnection,
    ) -> anyhow::Result<Vec<BalanceDb>> {
        balances::table
            .select(BalanceDb::as_select())
            .get_results(conn)
            .context("Failed to query balance by address")
    }
}
