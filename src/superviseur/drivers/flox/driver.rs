use std::{
    collections::HashMap,
    io::{BufRead, Write},
    process::{ChildStderr, ChildStdout},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Error;
use owo_colors::OwoColorize;
use spinners::{Spinner, Spinners};
use tokio::sync::mpsc;

use crate::{
    graphql::{
        schema::objects::subscriptions::{LogStream, TailLogStream},
        simple_broker::SimpleBroker,
    },
    superviseur::{
        core::ProcessEvent,
        drivers::DriverPlugin,
        logs::{Log, LogEngine},
    },
    types::{
        configuration::{DriverConfig, Service},
        process::Process,
    },
};

use nix::{
    sys::signal::{self, Signal},
    unistd::Pid,
};

#[derive(Clone)]
pub struct Driver {
    project: String,
    service: Service,
    processes: Arc<Mutex<Vec<(Process, String)>>>,
    childs: Arc<Mutex<HashMap<String, i32>>>,
    event_tx: mpsc::UnboundedSender<ProcessEvent>,
    log_engine: LogEngine,
}

impl Default for Driver {
    fn default() -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();
        Self {
            project: "".to_string(),
            service: Service::default(),
            processes: Arc::new(Mutex::new(Vec::new())),
            childs: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            log_engine: LogEngine::new(),
        }
    }
}

impl Driver {
    pub fn new(
        project: String,
        service: &Service,
        processes: Arc<Mutex<Vec<(Process, String)>>>,
        event_tx: mpsc::UnboundedSender<ProcessEvent>,
        childs: Arc<Mutex<HashMap<String, i32>>>,
        log_engine: LogEngine,
    ) -> Self {
        Self {
            project,
            service: service.clone(),
            processes,
            childs,
            event_tx,
            log_engine,
        }
    }

    pub fn setup_flox_env(&self, cfg: &DriverConfig) -> Result<(), Error> {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("flox --version")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("flox is not installed, see https://floxdev.com/docs/");

        let command = format!(
            "flox print-dev-env -A {}",
            cfg.environment.clone().unwrap().replace(".#", "")
        );
        let child = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        child.wait_with_output()?;
        Ok(())
    }

    pub fn write_logs(&self, stdout: ChildStdout, stderr: ChildStderr) {
        let cloned_service = self.service.clone();
        let log_engine = self.log_engine.clone();
        let project = self.project.clone();

        thread::spawn(move || {
            let service = cloned_service;
            let id = service.id.unwrap_or("-".to_string());
            // write stdout to file
            let mut log_file = std::fs::File::create(service.stdout).unwrap();

            let stdout = std::io::BufReader::new(stdout);
            for line in stdout.lines() {
                let line = line.unwrap();
                let line = format!("{}\n", line);

                let log = Log {
                    project: project.clone(),
                    service: service.name.clone(),
                    line: line.clone(),
                    output: String::from("stdout"),
                    date: tantivy::DateTime::from_timestamp_secs(chrono::Local::now().timestamp()),
                };
                match log_engine.insert(&log) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error while inserting log: {}", e);
                    }
                }

                SimpleBroker::publish(TailLogStream {
                    id: id.clone(),
                    line: line.clone(),
                });
                SimpleBroker::publish(LogStream {
                    id: id.clone(),
                    line: line.clone(),
                });
                let service_name = format!("{} | ", service.name);
                print!("{} {}", service_name.cyan(), line);
                log_file.write_all(line.as_bytes()).unwrap();
            }

            // write stderr to file
            let mut err_file = std::fs::File::create(service.stderr).unwrap();
            let stderr = std::io::BufReader::new(stderr);
            for line in stderr.lines() {
                let line = line.unwrap();

                let log = Log {
                    project: project.clone(),
                    service: service.name.clone(),
                    line: line.clone(),
                    output: String::from("stderr"),
                    date: tantivy::DateTime::from_timestamp_secs(chrono::Local::now().timestamp()),
                };
                match log_engine.insert(&log) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error while inserting log: {}", e);
                    }
                }

                err_file.write_all(line.as_bytes()).unwrap();
            }
        });
    }
}

impl DriverPlugin for Driver {
    fn start(&self, project: String) -> Result<(), Error> {
        let cfg = self
            .service
            .r#use
            .as_ref()
            .unwrap()
            .into_iter()
            .find(|(driver, _)| *driver == "flox")
            .map(|(_, x)| x)
            .unwrap();
        let message = format!(
            "Setup flox environment {} ...",
            cfg.environment.clone().unwrap()
        );
        let mut sp = Spinner::new(Spinners::Line, message.into());
        if self.setup_flox_env(&cfg).is_err() {
            println!("There is an error with flox env");
            return Ok(());
        }
        sp.stop();
        println!("\nSetup flox env done !");

        let command = format!(
            "flox activate -e {} -- {}",
            cfg.environment.clone().unwrap(),
            &self.service.command
        );
        println!("command: {}", command);

        let envs = self.service.env.clone();
        let working_dir = self.service.working_dir.clone();
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .envs(envs)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut processes = self.processes.lock().unwrap();

        let mut process = &mut processes
            .iter_mut()
            .find(|(p, key)| p.name == self.service.name && key == &project)
            .unwrap()
            .0;
        process.pid = Some(child.id());
        process.up_time = Some(chrono::Utc::now());
        let service_key = format!("{}-{}", project, self.service.name);
        self.childs
            .lock()
            .unwrap()
            .insert(service_key, process.pid.unwrap() as i32);

        self.event_tx.send(ProcessEvent::Started(
            self.service.name.clone(),
            project.clone(),
        ))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        self.write_logs(stdout, stderr);
        Ok(())
    }

    fn stop(&self, project: String) -> Result<(), Error> {
        if let Some(stop_command) = self.service.stop_command.clone() {
            let envs = self.service.env.clone();
            let working_dir = self.service.working_dir.clone();
            let cfg = self
                .service
                .r#use
                .as_ref()
                .unwrap()
                .into_iter()
                .find(|(driver, _)| *driver == "flox")
                .map(|(_, x)| x)
                .unwrap();

            let stop_command = format!(
                "flox activate -e {} -- {}",
                cfg.environment.clone().unwrap(),
                stop_command
            );
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(stop_command)
                .current_dir(working_dir)
                .envs(envs)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            child.wait().unwrap();

            let mut childs = self.childs.lock().unwrap();
            let service_key = format!("{}-{}", project.clone(), self.service.name.clone());
            childs.remove(&service_key);

            self.event_tx
                .send(ProcessEvent::Stopped(
                    self.service.name.clone(),
                    project.clone(),
                ))
                .unwrap();

            return Ok(());
        }

        let mut childs = self.childs.lock().unwrap();
        let service_key = format!("{}-{}", project.clone(), self.service.name.clone());

        match childs.get(&service_key) {
            Some(pid) => {
                println!(
                    "Stopping service {} (pid: {})",
                    self.service.name.clone(),
                    pid
                );
                signal::kill(Pid::from_raw(*pid), Signal::SIGTERM)?;
                childs.remove(&service_key);

                self.event_tx
                    .send(ProcessEvent::Stopped(
                        self.service.name.clone(),
                        project.clone(),
                    ))
                    .unwrap();

                println!("Service {} stopped", self.service.name);
                Ok(())
            }
            None => {
                println!("Service {} is not running", self.service.name);
                Ok(())
            }
        }
    }

    fn restart(&self, project: String) -> Result<(), Error> {
        self.stop(project.clone())?;
        self.start(project)?;
        Ok(())
    }

    fn status(&self) -> Result<(), Error> {
        Ok(())
    }

    fn logs(&self) -> Result<(), Error> {
        Ok(())
    }

    fn exec(&self) -> Result<(), Error> {
        Ok(())
    }

    fn build(&self, project: String) -> Result<(), Error> {
        if let Some(build) = self.service.build.clone() {
            let envs = self.service.env.clone();
            let working_dir = self.service.working_dir.clone();
            let cfg = self
                .service
                .r#use
                .as_ref()
                .unwrap()
                .into_iter()
                .find(|(driver, _)| *driver == "flox")
                .map(|(_, x)| x)
                .unwrap();
            self.setup_flox_env(&cfg)?;

            let build_command = format!(
                "flox activate -e {} -- {}",
                cfg.environment.clone().unwrap(),
                build.command
            );
            println!("build_command: {}", build_command);
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(build_command)
                .current_dir(working_dir)
                .envs(envs)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            self.write_logs(stdout, stderr);
            child.wait()?;

            self.event_tx
                .send(ProcessEvent::Built(
                    self.service.name.clone(),
                    project.clone(),
                ))
                .unwrap();
        }
        Ok(())
    }
}
