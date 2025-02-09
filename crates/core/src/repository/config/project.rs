use badge_maker::Style;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use super::RepositoryConfigType;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct ProjectConfig {
    #[schemars(title = "Badge Settings")]
    pub badge_settings: BadgeSettings,
    /// Whether to require a semver version for releases
    pub require_semver: bool,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectConfigType;
impl RepositoryConfigType for ProjectConfigType {
    fn get_type(&self) -> &'static str {
        "project"
    }
    fn get_description(&self) -> super::ConfigDescription {
        super::ConfigDescription {
            name: "Project",
            description: Some("Project settings for the repository"),
            documentation_link: None,
            ..Default::default()
        }
    }
    fn validate_config(
        &self,
        config: serde_json::Value,
    ) -> Result<(), super::RepositoryConfigError> {
        let _config: ProjectConfig = serde_json::from_value(config)?;
        Ok(())
    }
    fn default(&self) -> Result<serde_json::Value, super::RepositoryConfigError> {
        Ok(serde_json::to_value(ProjectConfig::default())?)
    }
    fn schema(&self) -> Option<schemars::Schema> {
        Some(schema_for!(ProjectConfig))
    }
    fn get_type_static() -> &'static str
    where
        Self: Sized,
    {
        "project"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BadgeSettings {
    pub style: BadgeStyle,
    pub label_color: String,
    pub color: String,
}
impl Default for BadgeSettings {
    fn default() -> Self {
        BadgeSettings {
            style: Default::default(),
            label_color: "#555".parse().unwrap(),
            color: "#33B5E5".parse().unwrap(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeStyle(pub Style);
impl Default for BadgeStyle {
    fn default() -> Self {
        BadgeStyle(Style::Flat)
    }
}

impl schemars::JsonSchema for BadgeStyle {
    fn schema_id() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("BadgeStyle")
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("BadgeStyle")
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        {
            let mut map = schemars::_serde_json::Map::new();
            map.insert("type".to_owned(), "string".into());
            map.insert(
                "enum".to_owned(),
                serde_json::Value::Array({
                    vec!["flat".into(), "plastic".into(), "flatquare".into()]
                }),
            );
            schemars::Schema::from(map)
        }
    }
}
