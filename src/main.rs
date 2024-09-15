//! The Rust Berlin Hack and Learn Ressource Library
//!
//! Webframework : Rocket
//! Database     : SQLite
//! SQL-Interface: sqlx
//!
//! Author: Andreas Klostermaier

// Dependencies

use std::path::{Path, PathBuf};

// Rocket related
#[macro_use] extern crate rocket;

use rocket::{fs::NamedFile, response::status::NotFound, shield::Shield};
use rocket_db_pools::{Connection, Database};
use rocket_dyn_templates::Template;

use halreslib::{url::{fetch_url_index, import_urls}, HaLdb};

#[get("/")]
async fn index(db: Connection<HaLdb>) -> Template {
    let mut context = std::collections::HashMap::new();
    context.insert("user_uuid", "anonymous".to_string());
    let url_list = fetch_url_index(db).await.expect("[URL] Fetching url index failed.");
    context.insert("urls", url_list);

    Template::render("start/start", &context)
}

/// This route returns static files of the webclient
#[get("/www/static/<file..>")]
async fn static_files(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("www/static/").join(file);
    NamedFile::open(&path).await.map_err(|e| NotFound(e.to_string()))
}

#[get("/status")]
fn status() -> &'static str {
    "Up and running..."
}

#[get("/api/import-urls")]
async fn api_import_urls(db: Connection<HaLdb>) -> &'static str {
    import_urls(db).await.unwrap();
    "URL import complete!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(HaLdb::init())
        .mount("/", routes![
            index,
            static_files,
            status,
            api_import_urls,
        ])
        .attach(Shield::new())
        .attach(Template::fairing())
}
