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
    pub build: Option<Build>,

    #[serde(
        rename = "use",
        skip_serializing_if = "Option::is_none",
        serialize_with = "hcl::ser::labeled_block"
    )]
    pub r#use: Option<IndexMap<String, DriverConfig>>,

    #[serde(
        rename = "deploy",
        skip_serializing_if = "Option::is_none",
        serialize_with = "hcl::ser::labeled_block"
    )]
    pub deploy: Option<DeployConfig>,

    #[serde(
        rename = "test",
        skip_serializing_if = "Option::is_none",
        serialize_with = "hcl::ser::labeled_block"
    )]
    pub test: Option<TestConfig>,
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
pub struct Build {
    pub command: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DriverConfig {
    pub environment: Option<String>,  // flox, nix
    pub image: Option<String>,        // docker, podman
    pub volumes: Option<Vec<String>>, // docker, podman
    pub ports: Option<Vec<String>>,   // docker, podman
    pub network: Option<String>,      // docker, podman
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DeployConfig {}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TestConfig {
    pub command: String,
}
