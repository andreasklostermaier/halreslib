/// Logging and Error handling
use log::{debug, trace, warn};
use anyhow::{Context, Result};

// Serde
use rocket::serde::{Deserialize, Serialize};

// Rocket DB-Pool
use rocket_db_pools::{sqlx, Connection};
use sqlx::Acquire;
//use rocket_db_pools::sqlx::sqlite::SqliteRow;
//use rocket_db_pools::sqlx::Row;

use reqwest::Url;
use select::document::Document;
use select::predicate::{Attr, Name};

use std::{fs::File, io::BufReader};

use crate::HaLdb;


// Module API responses
#[derive(Serialize, Deserialize, Debug)]
pub enum UrlLibResponse {
    UrlFileImportSuccess,
    UrlFileImportFailure,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
#[derive(sqlx::FromRow)]
pub struct URI {
    uri_uuid    : String,
    url         : String,
    scheme      : String,
    host        : String,
    path        : String,
    live_status : String,
    title       : String,
    auto_descr  : String,
    man_descr   : String,
    crea_user   : String,
    crea_time   : String,
    modi_user   : String,
    modi_time   : String,
}


#[derive(Debug, Deserialize, Serialize)]
struct Entry {
    date: chrono::NaiveDate,
    url: Url,
}


impl From<Entry> for URI {
    fn from(Entry { date, url }: Entry) -> Self {
        URI {
            uri_uuid    : blake3::hash(url.as_str().as_bytes()).to_hex().to_string(),
            url         : url.as_str().into(),
            scheme      : url.scheme().into(),
            host        : url.host_str().unwrap_or("-").into(),
            path        : url.path().into(),
            live_status : "1".to_string(),
            title       : "-".to_string(),
            auto_descr  : "-".to_string(),
            man_descr   : "".to_string(),
            crea_time   : date.to_string(), // should stay datetime, probably.
            crea_user   : "api".to_string(),
            modi_time   : "".to_string(),
            modi_user   : "api".to_string(),
        }
    }
}


pub async fn fetch_url_index(mut db: Connection<HaLdb>) -> Result<String> {
    let urls: Vec<URI> = sqlx::query_as("
        SELECT *
        FROM uris
        ORDER BY host, title;")
    .fetch_all(&mut **db)
    .await
    .context("[URI] SQL: Fetching url list failed!")?;

    Ok(serde_json::to_string(&urls).unwrap())
}


pub async fn import_urls(mut db: Connection<HaLdb>) -> Result<UrlLibResponse> {
    let path = "exchange/urls/urls.csv";

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(BufReader::new(
            File::open(path).context("Failed to open CSV file")?,
        ));

    // Begin SQL transaction
    let mut transaction = db.begin().await
        .context("PANIC! Unable to begin SQL transaction.")?;

    for entry in reader.deserialize::<Entry>() {
        let Ok(entry) = entry else {
            debug!("Failed to read line: {entry:?}");
            continue;
        };

        let mut page_info = URI::from(entry);

        debug!("Checking URL: {}", page_info.url);
        poke_page(&mut page_info).await?;

        // Write to database
        if let Err(error) = sqlx::query("INSERT INTO uris values (?,?,?,?,?,?,?,?,?,?,?,?,?);")
            .bind(&page_info.uri_uuid)
            .bind(&page_info.url)
            .bind(&page_info.scheme)
            .bind(&page_info.host)
            .bind(&page_info.path)
            .bind(&page_info.live_status)
            .bind(&page_info.title)
            .bind(&page_info.auto_descr)
            .bind(&page_info.man_descr)
            .bind(&page_info.crea_user)
            .bind(&page_info.crea_time)
            .bind(&page_info.modi_user)
            .bind(&page_info.modi_time)
            .execute(&mut *transaction)
            .await
        {
            warn!("Insertion failed: {error}");
        }

        trace!("{:#?}", page_info);
    }

    transaction
        .commit()
        .await
        .context("PANIC! Unable to commit SQL transaction.")?;

    Ok(UrlLibResponse::UrlFileImportSuccess)
}


async fn poke_page(page_info: &mut URI) -> Result<(), anyhow::Error> {
    match reqwest::get(&page_info.url).await {
        Ok(response) if response.status().is_success() => {
            let body = response.text().await?;
            let document = Document::from(body.as_str());
            if let Some(title) = document.find(Name("title")).next() {
                page_info.title = title.text();
            }
            if let Some(description) = document.find(Attr("name", "description")).next() {
                if let Some(content) = description.attr("content") {
                    page_info.auto_descr = content.to_string();
                }
            }
        }
        Ok(response) => warn!(
            "Error {} while retrieving:\n  {}",
            response.status(),
            page_info.url
        ),
        Err(error) => {
            page_info.live_status = "0".to_string();
            warn!("No response from URL ({error}):\n  {}", page_info.url);
        }
    }
    Ok(())
}
