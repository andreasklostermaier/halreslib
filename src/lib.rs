pub mod url;

use rocket_db_pools::{sqlx, Database};

#[derive(Database)]
#[database("halreslib_database")]
pub struct HaLdb(sqlx::SqlitePool);
