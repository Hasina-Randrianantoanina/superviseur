use std::{collections::HashMap, io::Write};

use indexmap::IndexMap;
use owo_colors::OwoColorize;

use crate::types::configuration::{ConfigFormat, ConfigurationData, Service};

pub fn execute_new(cfg_format: ConfigFormat) {
    let mut env = HashMap::new();
    env.insert("GITHUB_DOMAIN".to_string(), "github.com".to_string());

    let mut services = IndexMap::new();
    services.insert(
        String::from("demo"),
        Service {
            id: None,
            name: "demo".to_string(),
            r#type: "exec".to_string(),
            command: "ping $GITHUB_DOMAIN".to_string(),
            stop_command: None,
            working_dir: "/tmp".to_string(),
            watch_dir: None,
            description: Some("Ping Service Example".to_string()),
            depends_on: vec![],
            dependencies: vec![],
            env,
            autostart: true,
            autorestart: false,
            namespace: Some("demo_namespace".to_string()),
            port: None,
            stdout: "/tmp/demo-stdout.log".to_string(),
            stderr: "/tmp/demo-stderr.log".to_string(),
            wait_for: None,
            build: None,
            r#use: None,
            deploy: None,
            test: None,
        },
    );

    let config = ConfigurationData {
        project: "demo".to_string(),
        context: None,
        services,
    };
    let serialized = match cfg_format {
        ConfigFormat::HCL => hcl::to_string(&config).unwrap(),
        ConfigFormat::TOML => toml::to_string_pretty(&config).unwrap(),
    };

    let ext = match cfg_format {
        ConfigFormat::HCL => "hcl",
        ConfigFormat::TOML => "toml",
    };

    let filename = format!("Superfile.{}", ext);
    let mut file = std::fs::File::create(&filename).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();
    println!("Created {} ✨", filename.bright_green());
}
