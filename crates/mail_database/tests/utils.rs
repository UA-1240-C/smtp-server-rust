use mail_database::PgMailDB;
use diesel::prelude::*;
use diesel::pg::PgConnection;

pub struct TestContext {
    pub base_url: String,
    pub db_name: String,
    pub pg_db: PgMailDB,
}

impl TestContext {
    pub fn new(base_url: &str, db_name: &str) -> Self {
        let postgres_url = format!("{}/postgres", base_url);
        let mut conn =
            PgConnection::establish(&postgres_url).expect("Cannot connect to postgres database.");

        // Create a new database for the test
        let query = diesel::sql_query(format!("CREATE DATABASE {}", db_name).as_str());
        query
            .execute(&mut conn)
            .expect(format!("Could not create database {}", db_name).as_str());

        Self {
                base_url: base_url.to_string(),
                db_name: db_name.to_string(),
                pg_db: PgMailDB::new("testhost".to_string())
        }
    }    
    pub fn get_connection_string(&self) -> String {
        format!("{}/{}", self.base_url, self.db_name)
    }    
}

impl Drop for TestContext {

    fn drop(&mut self) {
        let postgres_url = format!("{}/postgres", self.base_url);
        let mut conn =
            PgConnection::establish(&postgres_url).expect("Cannot connect to postgres database.");

        let disconnect_users = format!(
            "SELECT pg_terminate_backend(pid)
                FROM pg_stat_activity
                WHERE datname = '{}';",
            self.db_name
        );

        diesel::sql_query(disconnect_users.as_str())
            .execute(&mut conn)
            .unwrap();


        let query = diesel::sql_query(format!("DROP DATABASE {}", self.db_name).as_str());
        query
            .execute(&mut conn)
            .expect(&format!("Couldn't drop database {}", self.db_name));
    }
}
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn setup_database(base_url: &str, db_name: &str) -> (TestContext, PgConnection) {
    let res = TestContext::new(base_url, db_name);
    let postgres_url = format!("{}/{}", res.base_url, res.db_name);
    let mut conn =
        PgConnection::establish(&postgres_url).expect("Cannot connect to postgres database.");
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    (res, conn)
}
