#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;
extern crate strum;
extern crate strum_macros;
extern crate core;

use std::fs::read_to_string;
use actix_cors::Cors;
use actix_web::web::{Data, PayloadConfig};
use actix_web::{middleware, web, App, HttpServer};
use std::path::Path;

use log::{error, info};
use nitro_log::config::Config;
use nitro_log::NitroLogger;
use std::sync::Mutex;

use crate::api_response::{APIResponse, SiteResponse};
use crate::utils::Resources;

pub mod database;

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

use crate::database::Database;
use crate::install::load_installer;
use clap::Parser;
use crossterm::style::Stylize;
use crate::settings::models::{EmailSetting, GeneralSettings, Mode, MysqlSettings, SecuritySettings, Settings, SiteSetting, StringMap};
use crate::storage::models::{load_storages, Storages};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    install: bool,
}

#[derive(Debug)]
pub struct NitroRepo {
    storages: Mutex<Storages>,
    settings: Mutex<Settings>,
    core: GeneralSettings,
}

pub type NitroRepoData = Data<NitroRepo>;


fn load_configs() -> anyhow::Result<Settings> {
    let cfgs = Path::new("cfg");

    let security: SecuritySettings = toml::from_str(&read_to_string(cfgs.join("security.toml"))?)?;
    let site: SiteSetting = toml::from_str(&read_to_string(cfgs.join("site.toml"))?)?;
    let email: EmailSetting = toml::from_str(&read_to_string(cfgs.join("email.toml"))?)?;

    Ok(Settings {
        email,
        site,
        security,
    })
}

fn load_logger<T: AsRef<Mode>>(logger: T) {
    let file = match logger.as_ref() {
        Mode::Debug => "log-debug.json",
        Mode::Release => "log-release.json",
        Mode::Install => "log-install.json",
    };
    let config: Config = serde_json::from_str(Resources::file_get_string(file).as_str()).unwrap();
    NitroLogger::load(config, None).unwrap();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let path = Path::new("cfg");
    let main_config = path.join("nitro_repo.toml");
    if !main_config.exists() {
        let parse: Cli = Cli::parse();
        if parse.install {
            load_logger(Mode::Install);
            if let Err(error) = load_installer(path) {
                error!("Unable to complete Install {error}");
                println!("{}", "Unable to Complete Installation".red());
                std::process::exit(1);
            }
            println!("{}", "Installation Complete".green());
            return Ok(());
        } else {
            println!(
                "Nitro Repo Not Installed. Please ReRun nitro launcher with the --install flag"
            );
            std::process::exit(1);
        }
    }
    info!("Loading Main Config");
    let string = read_to_string(&main_config)?;
    let init_settings: GeneralSettings = toml::from_str(&string)?;
    // Sets the Log Location
    std::env::set_var("LOG_LOCATION", &init_settings.application.log);

    load_logger(&init_settings.application.mode);

    info!("Initializing Database");
    let pool = match init_settings.database.db_type.as_str() {
        "mysql" => {
            let result = MysqlSettings::try_from(init_settings.database.settings.clone());
            if let Err(error) = result {
                error!("Unable to load database Settings {error}");
                std::process::exit(1);
            }
            database::init(&result.unwrap().to_string())
        }
        _ => {
            error!("Invalid Database Type");
            std::process::exit(1);
        }
    };
    if let Err(error) = pool {
        error!("Unable to load database {error}");
        std::process::exit(1);
    }
    let pool = pool.unwrap();
    info!("Loading Other Configs");
    let settings = load_configs();
    if let Err(error) = settings {
        error!("Unable to load Settings {error}");
        std::process::exit(1);
    }
    let settings = settings.unwrap();
    info!("Loading Storages");
    let storages = load_storages();
    if let Err(error) = storages {
        error!("Unable to load Settings {error}");
        std::process::exit(1);
    }
    let storages = storages.unwrap();
    info!("Initializing Web Server");
    let nitro_repo = NitroRepo {
        storages: Mutex::new(storages),
        settings: Mutex::new(settings),
        core: init_settings,
    };
    let max_upload = nitro_repo.core.application.max_upload;
    let address = nitro_repo.core.application.address.clone();
    let data = web::Data::new(nitro_repo);

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
            .app_data(data.clone())
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
                .bind_openssl(address, builder)?
                .run()
                .await;
        }
    }

    return server.bind(address)?.run().await;
}


