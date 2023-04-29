use crate::state::read_property;
use crate::state::{save::exists, SaveError};
use crate::utils::{append_comma_separated, append_json_string};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::ChildStdin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

lazy_static! {
    static ref INSTANCES: RwLock<HashMap<String, Instance>> = RwLock::new(HashMap::new());
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Offline,
    Loading,
    Online,
    Shutdown,
}

struct Instance {
    status: InstanceStatus,
    port: u16,
    stdin: Arc<Mutex<ChildStdin>>,
}

impl Instance {
    fn stop(&self) -> Result<(), SaveError> {
        match self.status {
            InstanceStatus::Offline => unreachable!(),
            InstanceStatus::Loading => Err(SaveError::IsLoading),
            InstanceStatus::Online => {
                let mut stdin = self.stdin.lock().unwrap();
                if stdin.write(b"stop\r\n").is_ok() {
                    Ok(())
                } else {
                    Err(SaveError::IOError)
                }
            }
            InstanceStatus::Shutdown => Ok(()),
        }
    }
}

/// creates the instance, returns an error if it is already online
pub fn start_instance(name: &str) -> Result<(), SaveError> {
    exists(name)?;
    let port = match read_property(format!("saves/{name}/server.properties"), "server-port")? {
        Some(port) => match port.parse() {
            Ok(port) => port,
            Err(_) => return Err(SaveError::IOError),
        },
        None => return Err(SaveError::IOError),
    };
    let mut instances = INSTANCES.write().unwrap();
    if instances.contains_key(name) {
        return Err(SaveError::IsOnline);
    }
    if instances.iter().any(|x| x.1.port == port) {
        return Err(SaveError::PortInUse);
    }
    let mut directory = std::env::current_dir().expect("current_dir");
    directory.push("saves");
    directory.push(name);
    let mut directory = directory.to_str().expect("failed to convert path to utf8");
    if directory.starts_with(r"\\?\") {
        directory = &directory[4..];
    }
    let mut child = std::process::Command::new("java.exe")
        .args(["-jar", "server.jar", "nogui"])
        .current_dir(directory)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn java.exe");
    println!("[{name}] Java process spawned");
    let stdin = Arc::new(Mutex::new(child.stdin.take().unwrap()));
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let instance = Instance {
        status: InstanceStatus::Loading,
        port,
        stdin,
    };
    let name_arc = Arc::new(name.to_owned());
    let name = name_arc.clone();
    // waits for child to complete
    tokio::spawn(async move {
        println!("[{name}] Waiter thread spawned");
        match child.wait() {
            Ok(status) => match status.code() {
                Some(0) => println!("[{name}] Java process finished"),
                Some(status) => println!("[{name}] Java process finished with status {status}"),
                None => println!("[{name}] Java process finished with no status"),
            },
            Err(error) => {
                println!(
                    "[{name}] An error occoured when attempting to wait for Java process, {:?}",
                    error
                );
            }
        }
        let mut instances = INSTANCES.write().unwrap();
        instances
            .remove(&*name)
            .expect("Waiter thread finished, and its instance was removed");
        println!("[{name}] Waiter thread finished");
    });
    let name: Arc<String> = name_arc.clone();
    // reads and parses stdout
    tokio::spawn(async move {
        println!("[{name}] Reader thread spawned");
        for line in stdout.lines() {
            match line {
                Ok(line) => {
                    if line.contains(" Done ") {
                        let mut instances = INSTANCES.write().unwrap();
                        if let Some(instance) = instances.get_mut(name.as_str()) {
                            if instance.status == InstanceStatus::Loading {
                                instance.status = InstanceStatus::Online;
                                if SHUTDOWN.load(Ordering::Relaxed) {
                                    if instance.stop().is_err() {
                                        panic!("could not send stop command through stdin");
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    } else if line.contains(" Stopping ") {
                        let mut instances = INSTANCES.write().unwrap();
                        if let Some(instance) = instances.get_mut(name.as_str()) {
                            if instance.status == InstanceStatus::Loading
                                || instance.status == InstanceStatus::Online
                            {
                                instance.status = InstanceStatus::Shutdown;
                            }
                        } else {
                            break;
                        }
                    }
                    println!("[{name}] {}", line);
                }
                Err(error) => {
                    println!("[{name}] Error reading process stdin: {:?}", error);
                    break;
                }
            }
        }
        println!("[{name}] Reader thread finished");
    });
    instances.insert(name_arc.to_string(), instance);
    Ok(())
}

/// stops the instance, returns immedialty, will return an error if it is not online
pub fn stop_instance(name: &str) -> Result<(), SaveError> {
    exists(name)?;
    let instances = INSTANCES.read().unwrap();
    if let Some(instance) = instances.get(name) {
        instance.stop()
    } else {
        Err(SaveError::IsOffline)
    }
}

/// checks if the instance is online, may returns an error if it is not online
pub fn query_instance(name: &str) -> Result<InstanceStatus, SaveError> {
    exists(name)?;
    let instances = INSTANCES.read().unwrap();
    if let Some(instance) = instances.get(name) {
        Ok(instance.status)
    } else {
        Ok(InstanceStatus::Offline)
    }
}

pub fn instance_status_summary() -> String {
    let mut out = String::with_capacity(4 * 1024);
    out.push('{');
    append_comma_separated(
        INSTANCES.read().unwrap().iter(),
        &mut out,
        |out, (name, instance)| {
            append_json_string(out, name);
            match instance.status {
                InstanceStatus::Offline => unreachable!(),
                InstanceStatus::Loading => *out += r#":"loading""#,
                InstanceStatus::Online => *out += r#":"online""#,
                InstanceStatus::Shutdown => *out += r#":"shutdown""#,
            }
        },
    );
    out.push('}');
    out
}

/// returns immediatly, signals to all instances that they must stop as soon as possible
pub fn stop_all_instances() {
    println!("[*] Shutting down all instances");
    for (_, instance) in INSTANCES.read().unwrap().iter() {
        if instance.status == InstanceStatus::Online {
            if instance.stop().is_err() {
                panic!("could not send stop command through stdin");
            }
        }
    }
}
