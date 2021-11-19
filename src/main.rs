#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;
extern crate strum;
extern crate strum_macros;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{App, HttpServer, middleware};
use actix_web::web::PayloadConfig;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use log::info;
use nitro_log::config::Config;
use nitro_log::NitroLogger;

use crate::install::install::Installed;
use crate::utils::Resources;

pub mod api_response;
pub mod error;
pub mod frontend;
pub mod install;
pub mod repository;
pub mod schema;
pub mod settings;
pub mod storage;
pub mod system;
pub mod utils;

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;
embed_migrations!();

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(error) = dotenv::dotenv() {
        println!("Unable to load dotenv {}", error);
        return Ok(());
    }
    let file = match std::env::var("MODE")
        .unwrap_or("DEBUG".to_string())
        .as_str()
    {
        "DEBUG" => "log-debug.json",
        "RELEASE" => "log-release.json",
        _ => {
            panic!("Must be Release or Debug")
        }
    };
    let config: Config =
        serde_json::from_str(Resources::file_get_string(file).as_str()).unwrap();
    NitroLogger::load(config, None).unwrap();
    info!("Initializing Database");
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<MysqlConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let connection = pool.get().unwrap();
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();
    std::env::set_var("INSTALLED", "false");
    info!("Initializing Web Server");
    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_header()
                    .allow_any_method()
                    .allow_any_origin(),
            )
            .wrap(middleware::Logger::default())
            .data(pool.clone())
            .wrap(Installed)
            .data(PayloadConfig::new(1 * 1024 * 1024 * 1024))
            .configure(error::handlers::init)
            .configure(settings::init)
            .configure(repository::init)
            .configure(storage::admin::init)
            .configure(repository::admin::init)
            .configure(install::init)
            .configure(system::controllers::init)
            .configure(frontend::init)
            // TODO Make sure this is the correct way of handling vue and actix together. Also learn about packaging the website.
            .service(Files::new("/", std::env::var("SITE_DIR").unwrap()).show_files_listing())
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
