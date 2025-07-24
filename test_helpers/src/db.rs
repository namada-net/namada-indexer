use std::convert::identity;
use std::sync::atomic::AtomicU32;
use std::{env, thread};

use deadpool_diesel::postgres::Pool;
use diesel::{Connection, PgConnection, RunQueryDsl, sql_query};
use orm::migrations::CustomMigrationSource;
use shared::error::{AsDbError, ContextDbInteractError};

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct TestDb {
    default_db_url: String,
    name: String,
    pool: Pool,
}

impl TestDb {
    pub fn new() -> Self {
        let name = format!(
            "test_db_{}_{}",
            std::process::id(),
            TEST_DB_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        let default_db_url =
            env::var("DATABASE_URL_TEST").expect("DATABASE_URL_TEST not set");
        let mut conn = PgConnection::establish(&default_db_url).unwrap();

        sql_query(format!("CREATE DATABASE {};", name))
            .execute(&mut conn)
            .expect("Failed to create test db");

        // TODO: this pool stuff is copied from AppState
        let max_pool_size = env::var("DATABASE_POOL_SIZE")
            .unwrap_or_else(|_| 8.to_string())
            .parse::<usize>()
            .unwrap_or(8_usize);

        let db_path = format!("{}/{}", default_db_url, name);

        let pool_manager = deadpool_diesel::Manager::new(
            db_path,
            deadpool_diesel::Runtime::Tokio1,
        );

        let pool = Pool::builder(pool_manager)
            .max_size(max_pool_size)
            .build()
            .expect("Failed to build Postgres db pool");

        Self {
            default_db_url: default_db_url.to_string(),
            name,
            pool,
        }
    }

    pub async fn run_test(
        &self,
        test: impl Fn(&mut PgConnection) -> anyhow::Result<()> + Send + 'static,
    ) -> anyhow::Result<()> {
        let conn = &mut self.pool.get().await?;

        CustomMigrationSource::new("test_chain_id".to_string())
            .run_migrations(conn)
            .await
            .expect("Should be able to run migrations");

        conn.interact(move |conn| {
            conn.build_transaction().read_write().run(test)
        })
        .await
        .context_db_interact_error()
        .and_then(identity)
        .into_db_error()?;

        anyhow::Ok(())
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        if thread::panicking() {
            eprintln!("TestDb leaking database {}", self.name);
            return;
        }
        let mut conn = PgConnection::establish(&self.default_db_url)
            .expect("Failed to connect to default db");
        sql_query(format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE \
             datname = '{}'",
            self.name
        ))
        .execute(&mut conn)
        .expect("Failed to terminate connections to test db");
        sql_query(format!("DROP DATABASE {}", self.name))
            .execute(&mut conn)
            .expect("Failed to drop test db");
    }
}

impl Default for TestDb {
    fn default() -> Self {
        Self::new()
    }
}
