use anyhow::Context;
use diesel::upsert::excluded;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use orm::pos_rewards::PosRewardInsertDb;
use orm::schema::{pos_rewards, validators};
use shared::rewards::Reward;
use shared::tuple_len::TupleLen;

use super::utils::MAX_PARAM_SIZE;

pub fn upsert_rewards(
    transaction_conn: &mut PgConnection,
    rewards: Vec<Reward>,
    epoch: i32,
) -> anyhow::Result<()> {
    let rewards_col_count = pos_rewards::all_columns.len() as i64;

    for chunk in rewards
        .into_iter()
        .collect::<Vec<_>>()
        .chunks((MAX_PARAM_SIZE as i64 / rewards_col_count) as usize)
    {
        upsert_rewards_chunk(transaction_conn, chunk.to_vec(), epoch)?;
    }

    anyhow::Ok(())
}

fn upsert_rewards_chunk(
    transaction_conn: &mut PgConnection,
    rewards: Vec<Reward>,
    epoch: i32,
) -> anyhow::Result<()> {
    diesel::insert_into(pos_rewards::table)
        .values::<Vec<PosRewardInsertDb>>(
            rewards
                .into_iter()
                .map(|reward| {
                    let validator_id: i32 = validators::table
                        .filter(
                            validators::namada_address.eq(&reward
                                .delegation_pair
                                .validator_address
                                .to_string()),
                        )
                        .select(validators::id)
                        .first(transaction_conn)
                        .expect("Failed to get validator");

                    PosRewardInsertDb::from_reward(reward, validator_id, epoch)
                })
                .collect::<Vec<_>>(),
        )
        .on_conflict((
            pos_rewards::columns::owner,
            pos_rewards::columns::validator_id,
            pos_rewards::columns::epoch, // Add epoch to conflict target
        ))
        .do_update()
        .set(
            pos_rewards::columns::raw_amount
                .eq(excluded(pos_rewards::columns::raw_amount)),
        )
        .execute(transaction_conn)
        .context("Failed to upsert rewards in db")?;

    Ok(())
}
