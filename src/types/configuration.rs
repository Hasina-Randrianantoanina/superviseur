use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub enum ConfigFormat {
    TOML,
    HCL,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Service {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub name: String,
    pub r#type: String, // docker, podman, exec, wasm
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_command: Option<String>,
    pub working_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_dir: Option<String>,
    pub description: Option<String>,
    pub depends_on: Vec<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub dependencies: Vec<String>,
    pub env: HashMap<String, String>,
    pub autostart: bool,
    pub autorestart: bool,
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
    pub stdout: String,
    pub stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flox: Option<Flox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<Build>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConfigurationData {
    pub project: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub context: Option<String>,
    #[serde(rename = "service", serialize_with = "hcl::ser::labeled_block")]
    pub services: IndexMap<String, Service>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Flox {
    pub environment: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Build {
    pub command: String,
}
