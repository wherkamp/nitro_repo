#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;
extern crate strum;
extern crate strum_macros;

use actix_cors::Cors;
use actix_web::web::PayloadConfig;
use actix_web::{middleware, web, App, HttpRequest, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use log::info;
use nitro_log::config::Config;
use nitro_log::NitroLogger;
use std::str::FromStr;

use crate::api_response::{APIResponse, SiteResponse};
use crate::utils::Resources;

pub mod api_response;
pub mod error;
pub mod frontend;
pub mod install;
mod misc;
pub mod repository;
pub mod schema;
pub mod settings;
pub mod storage;
pub mod system;
pub mod utils;
pub mod webhook;

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;
type Database = web::Data<DbPool>;
embed_migrations!();
fn load_logger(logger: &str){

    let file = match std::env::var("MODE")
        .expect("Mode Must be RELEASE OR DEBUG")
        .as_str()
    {
        "DEBUG" => "log-debug.json",
        "RELEASE" => "log-release.json",
        "INSTALL" => "log-install.json",
        _ => {
            panic!("Must be Release or Debug")
        }
    };
    let config: Config = serde_json::from_str(Resources::file_get_string(file).as_str()).unwrap();
    NitroLogger::load(config, None).unwrap();
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(error) = dotenv::dotenv() {
        println!("Unable to load dotenv {}", error);
        return Ok(());
    }
    let logger = std::env::var("MODE")
        .expect("Mode Must be RELEASE OR DEBUG");


    info!("Initializing Database");
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<MysqlConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let connection = pool.get().unwrap();
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();

    info!("Initializing Web Server");
    let max_upload = std::env::var("MAX_UPLOAD").unwrap_or_else(|_| "1024".to_string());
    let max_upload = i64::from_str(&max_upload).unwrap();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_header()
                    .allow_any_method()
                    .allow_any_origin(),
            )
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(PayloadConfig::new(
                (max_upload * 1024 * 1024) as usize,
            )))
            .configure(error::handlers::init)
            .configure(settings::init)
            .configure(repository::init)
            .configure(storage::admin::init)
            .configure(repository::admin::init)
            .configure(system::controllers::init)
            .configure(misc::init)
            .configure(frontend::init)
        // TODO Make sure this is the correct way of handling vue and actix together. Also learn about packaging the website.
    })
    .workers(2);

    // I am pretty sure this is correctly working
    // If I am correct this will only be available if the feature ssl is added
    #[cfg(feature = "ssl")]
    {
        if std::env::var("PRIVATE_KEY").is_ok() {
            use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

            let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
            builder
                .set_private_key_file(std::env::var("PRIVATE_KEY").unwrap(), SslFiletype::PEM)
                .unwrap();
            builder
                .set_certificate_chain_file(std::env::var("CERT_KEY").unwrap())
                .unwrap();
            return server
                .bind_openssl(std::env::var("ADDRESS").unwrap(), builder)?
                .run()
                .await;
        }
    }

    return server.bind(std::env::var("ADDRESS").unwrap())?.run().await;
}

#[actix_web::get("/api/installed")]
pub async fn installed(pool: Database, r: HttpRequest) -> SiteResponse {
    let connection = pool.get()?;
    let result = crate::utils::installed(&connection)?;
    APIResponse::new(true, Some(result)).respond(&r)
}
