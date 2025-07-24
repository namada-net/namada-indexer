use std::error::Error;

use deadpool_diesel::postgres::Object;
use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationSource};
use diesel::pg::Pg;
use diesel_migrations::{
    EmbeddedMigrations, MigrationHarness, embed_migrations,
};

pub const PREFIX: &str = "only_";
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub struct CustomMigrationSource<DB: Backend> {
    all_migrations: Box<dyn MigrationSource<DB> + Send + Sync>,
    chain_id: String,
}

impl CustomMigrationSource<Pg> {
    pub fn new(chain_id: String) -> Self {
        CustomMigrationSource {
            all_migrations: Box::new(MIGRATIONS),
            chain_id,
        }
    }

    pub async fn run_migrations(self, conn: &Object) -> Result<(), String> {
        conn.interact(|transaction_conn| {
            transaction_conn
                .run_pending_migrations(self)
                .expect("Failed to run all migrations");
        })
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
}

impl<DB: Backend> MigrationSource<DB> for CustomMigrationSource<DB> {
    fn migrations(
        &self,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Box<dyn Error + Send + Sync>> {
        let mut selected = Vec::new();

        for migration in self.all_migrations.migrations()? {
            let migration_name = migration.name().to_string();

            if migration_name.contains(PREFIX)
                && !migration_name.contains(&self.chain_id)
            {
                println!(
                    "Skipping migration {} for chain {}",
                    migration_name, self.chain_id
                );
                continue;
            }
            selected.push(migration)
        }

        Ok(selected)
    }
}
