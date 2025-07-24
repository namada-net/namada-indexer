use std::env;

use anyhow::Context;
use deadpool_diesel::postgres::{Object, Pool as DbPool};

#[derive(Clone)]
pub struct AppState {
    db: DbPool,
}

impl AppState {
    pub fn new(db_url: String) -> anyhow::Result<Self> {
        let max_pool_size = env::var("DATABASE_POOL_SIZE")
            .unwrap_or_else(|_| 8.to_string())
            .parse::<usize>()
            .unwrap_or(8_usize);
        let pool_manager = deadpool_diesel::Manager::new(
            db_url,
            deadpool_diesel::Runtime::Tokio1,
        );
        let pool = DbPool::builder(pool_manager)
            .max_size(max_pool_size)
            .build()
            .context("Failed to build Postgres db pool")?;

        Ok(Self { db: pool })
    }

    pub async fn get_db_connection(&self) -> anyhow::Result<Object> {
        self.db
            .get()
            .await
            .context("Failed to get db connection handle from deadpool")
    }
}
