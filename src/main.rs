#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate lazy_static_include;
extern crate strum;
extern crate strum_macros;

use std::collections::HashMap;
use std::env;
use std::path::Path;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    App, get, HttpRequest, HttpResponse, HttpServer, middleware, post, ResponseError, web,
};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use log4rs::config::RawConfig;
use log::info;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use tera::Tera;

use crate::apierror::APIError;
use crate::settings::settings::get_file;
use crate::utils::Resources;

pub mod api_response;
pub mod install;
pub mod schema;
pub mod settings;
pub mod apierror;
pub mod utils;
pub mod system;
pub mod storage;
pub mod repository;
pub mod internal_error;
pub mod site_response;
pub mod frontend;
pub mod error;

fn url(args: &HashMap<String, serde_json::Value>) -> Result<tera::Value, tera::Error> {
    let option = args.get("path");
    return if option.is_some() {
        let x = option.unwrap().to_string().replace("\"","");
        println!("{}", &x);
        let x1 = std::env::var("URL").unwrap();
        let string = format!("{}/{}", x1, x);
        println!("{}", &string);
        let result = tera::Value::from(&*string);
        Ok(result)
    } else {
        Err(tera::Error::from("Missing Param Tera".to_string()))
    };
}

fn url_raw(value: &str) -> String {
    let url = std::env::var("URL").unwrap();
    let string = format!("{}/{}", url, value);
    return string;
}

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;
embed_migrations!();
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    Command
    std::env::set_var("RUST_LOG", "actix_web=trace");
    std::env::set_var("RUST_BACKTRACE", "1");
    let config: RawConfig =
        serde_yaml::from_str(Resources::file_get_string("log.yml").as_str()).unwrap();
    log4rs::init_raw_config(config).unwrap();
    dotenv::dotenv().ok();
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<MysqlConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let connection = pool.get().unwrap();
    embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();
    info!("Test");

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_method()
            .allow_any_origin();
        let result1 = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/site/templates/**/*"));
        if result1.is_err() {
            println!("{}", result1.err().unwrap());
            panic!("Unable to create Tera")
        }
        let mut tera = result1.unwrap();
        tera.register_function("url", url);
        App::new()

            .wrap(cors)
            .wrap(middleware::Logger::default())
            .data(pool.clone())
            .data(tera)
            .service(install::installed)
            .service(settings::controller::update_setting)
            .service(settings::controller::about_setting)
            .service(repository::admin::controller::add_server)
            .service(repository::admin::controller::list_servers)
            .service(storage::admin::controller::add_server)
            .service(storage::admin::controller::list_storages)
            .configure(repository::init)
            .configure(frontend::install::init)
            .configure(frontend::public::init)
            .service(Files::new("/cdn", format!("{}/node_modules", std::env::var("SITE_DIR").unwrap())).show_files_listing())
            .service(Files::new("/", format!("{}/static", std::env::var("SITE_DIR").unwrap())).show_files_listing())
            .default_service(web::route().to(|| APIError::NotFound.error_response()))
    })
        .workers(2);
    if std::env::var("PRIVATE_KEY").is_ok() {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(std::env::var("PRIVATE_KEY").unwrap(), SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(std::env::var("CERT_KEY").unwrap())
            .unwrap();

        server.bind_openssl("0.0.0.0:6742", builder)?.run().await
    } else {
        server.bind("0.0.0.0:6742")?.run().await
    }
}
