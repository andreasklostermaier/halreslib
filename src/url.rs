/// Logging and Error handling
use log::{info, debug, warn, error};
use anyhow::{Result, Context};

// Serde
use rocket::serde::{Serialize, Deserialize};

// Rocket DB-Pool
use rocket_db_pools::{sqlx, Connection};
use sqlx::Acquire;
//use rocket_db_pools::sqlx::sqlite::SqliteRow;
//use rocket_db_pools::sqlx::Row;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use reqwest::Url;
use select::document::Document;
use select::predicate::{Name, Attr};

// DateTime
use chrono::Local;

use crate::HaLdb;


// Module API responses
#[derive(Serialize, Deserialize, Debug)]
pub enum UrlLibResponse {
    UrlFileImportSuccess,
    UrlFileImportFailure,
}

#[derive(Serialize, Deserialize, Debug)]
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


pub async fn fetch_url_index(mut db: Connection<HaLdb>)
    -> Result<String> {

        let urls: Vec<URI> = sqlx::query_as(r#"
            SELECT *
            FROM uris
            ORDER BY host, title
            ;"#)
        .fetch_all(&mut **db)
        .await
        .context("[URI] SQL: Fetching url list failed!")?;
    
        Ok(serde_json::to_string(&urls).unwrap())
    
}


pub async fn import_urls(mut db: Connection<HaLdb>) -> Result<UrlLibResponse> {

    let path = "exchange/urls/urls.csv";

    // Begin SQL transaction
    let mut transaction = db.begin().await
        .context("PANIC! Unable to begin SQL transaction.")?;

    if let Ok(lines) = read_lines(path) {

        for l in lines.into_iter().map_while(Result::ok) {

            let log_timestamp = Local::now().to_rfc3339();

            let parts: Vec<&str> = l.split('\t').collect();
            if parts.len() > 1 {
                let url_str = parts[1].trim();

                let mut uri_entry = URI {
                    uri_uuid    : "".to_string(),
                    url         : "-".to_string(),
                    scheme      : "-".to_string(),
                    host        : "-".to_string(),
                    path        : "-".to_string(),
                    live_status : "1".to_string(),
                    title       : "-".to_string(),
                    auto_descr  : "-".to_string(),
                    man_descr   : "".to_string(),
                    crea_user   : "api".to_string(),
                    crea_time   : log_timestamp.to_owned(),
                    modi_user   : "api".to_string(),
                    modi_time   : log_timestamp.to_owned(),
                };
    
                if let Ok(parsed_url) = Url::parse(url_str) {
                    //info!("Checking URL: {}", normalized_url);
                    uri_entry.url      = parsed_url.as_str().into();
                    uri_entry.uri_uuid = blake3::hash(uri_entry.url.as_bytes()).to_hex().to_string();
                    uri_entry.scheme   = parsed_url.scheme().into();
                    uri_entry.host     = parsed_url.host_str().unwrap_or("-").into();
                    uri_entry.path     = parsed_url.path().into();

                    if let Ok(response) = reqwest::get(&uri_entry.url).await {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            let document = Document::from(body.as_str());
                            if let Some(title) = document.find(Name("title")).next() {
                                uri_entry.title = title.text();
                            }
                            if let Some(description) = document.find(Attr("name", "description")).next() {
                                if let Some(content) = description.attr("content") {
                                    uri_entry.auto_descr = content.to_string();
                                }
                            }
                        } else {
                            warn!("Error {} while retrieving:\n  {}", response.status(), uri_entry.url);
                        }
                    } else {
                        uri_entry.live_status = "0".to_string();
                        warn!("No response from URL:\n  {}", uri_entry.url);
                    }


                    // Write to database
                    let _insert_result = sqlx::query(r#"
                    INSERT INTO uris values (?,?,?,?,?,?,?,?,?,?,?,?,?);
                    "#)
                        .bind(&uri_entry.uri_uuid)
                        .bind(&uri_entry.url)
                        .bind(&uri_entry.scheme)
                        .bind(&uri_entry.host)
                        .bind(&uri_entry.path)
                        .bind(&uri_entry.live_status)
                        .bind(&uri_entry.title)
                        .bind(&uri_entry.auto_descr)
                        .bind(&uri_entry.man_descr)
                        .bind(&uri_entry.crea_user)
                        .bind(&uri_entry.crea_time)
                        .bind(&uri_entry.modi_user)
                        .bind(&uri_entry.modi_time)
                        .execute(&mut *transaction).await;

                    //info!("DEBUG: Result of db insert = {:?}", _insert_result);

                    //println!("{:#?}", uri_entry);

                } else {
                    error!("Ill-formed URL: {}", url_str);
                }

            }
        }
    }

    transaction.commit()
    .await
    .context("PANIC! Unable to commit SQL transaction.")?;

    Ok(UrlLibResponse::UrlFileImportSuccess)
}


fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
