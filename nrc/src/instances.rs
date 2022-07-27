use crate::api::User;
use crate::configs::user::RepositoryInstance;
use crate::configs::{get_user_config, save_user_config};
use crate::Parser;
use inquire::{error::InquireResult, min_length, Confirm, MultiSelect, Password, Select, Text};
use serde::{Deserialize, Serialize};
use serde_json::json;
use style_term::DefaultColor::{Green, Red};
use style_term::{Color, EightBitColor, StyleString};
use uuid::Uuid;

#[derive(Debug, Parser)]
pub struct Instances {
    #[clap(default_value = "false")]
    pub skip_login: bool,
}

impl Instances {
    pub async fn execute(self) -> anyhow::Result<()> {
        let result = get_user_config()?;
        let reqwest = reqwest::ClientBuilder::new()
            .user_agent("Nitro Repository CLI")
            .build()
            .unwrap();
        for (name, instance) in result.repositories {
            println!("{}: {}", name, instance.url);

            let option = User::me(&reqwest, instance.url.clone(), &instance).await?;
            if let Some(v) = option {
                println!("{}", format!("{}", v.username).style().text_color(Green));
            } else {
                //TODO remove the instance
                println!("{}", "No user found.".style().text_color(Red));
            }
        }
        Ok(())
    }
}
