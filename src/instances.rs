use crate::properties::read_property;
use crate::server::is_shutdown;
use crate::state::save;
use crate::utils::{append_comma_separated, append_json_string, ApiError};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{watch, Mutex, RwLock};

lazy_static! {
    static ref INSTANCES: RwLock<HashMap<String, Instance>> = RwLock::new(HashMap::new());
}

static JAVA_PATH: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

struct Instance {
    status: InstanceStatus,
    port: u16,
    stdin: Arc<Mutex<ChildStdin>>,
    vector: Arc<InstanceVector>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    /// the initial status, indicates the instance has never run yet and has no console output
    Cold,
    /// set as soon as the java instance starts
    Loading,
    /// set after a "Done" line is detected, the server may now accept commands
    Online,
    /// set after a stop is issued, the server must not accept any commands
    Shutdown,
    /// set after shutdown is complete, there is console output
    Offline,
}

pub struct InstanceVector {
    sender: watch::Sender<(Vec<u8>, bool)>,
}

impl Instance {
    async fn stop(&mut self) -> Result<(), ApiError> {
        match self.status {
            InstanceStatus::Cold => unreachable!(),
            InstanceStatus::Loading => Err(ApiError::BadInstanceStatus(InstanceStatus::Loading)),
            InstanceStatus::Online => {
                let mut stdin = self.stdin.lock().await;
                stdin.write(b"stop\r\n").await?;
                self.status = InstanceStatus::Shutdown;
                Ok(())
            }
            InstanceStatus::Shutdown => Ok(()),
            InstanceStatus::Offline => Err(ApiError::BadInstanceStatus(InstanceStatus::Offline)),
        }
    }
}

impl InstanceStatus {
    pub fn to_error(self) -> ApiError {
        ApiError::BadInstanceStatus(self)
    }
}

impl InstanceVector {
    fn new() -> Self {
        let (sender, _) = tokio::sync::watch::channel((Vec::new(), true));
        InstanceVector { sender }
    }
    async fn finish(&self) {
        self.sender.send_modify(|(_, alive)| *alive = false);
    }
    async fn write(&self, data: &[u8]) {
        self.sender
            .send_modify(|(buffer, _)| buffer.extend_from_slice(data));
    }
    pub fn subscribe(&self) -> watch::Receiver<(Vec<u8>, bool)> {
        self.sender.subscribe()
    }
}

/// creates the instance, returns an error if it is already online
pub async fn start_instance(name: &str) -> Result<(), ApiError> {
    save::exists(name)?;
    let port = match read_property(format!("saves/{name}/server.properties"), "server-port")? {
        Some(port) => match port.parse() {
            Ok(port) => port,
            Err(_) => return Err(ApiError::BadConfig("server-port".to_owned())),
        },
        None => return Err(ApiError::BadConfig("server-port".to_owned())),
    };
    let mut instances = INSTANCES.write().await;
    if let Some(instance) = instances.get(name) {
        if instance.status != InstanceStatus::Offline {
            return Err(instance.status.to_error());
        }
    }
    if instances.iter().any(|x| x.1.port == port && matches!(x.1.status, InstanceStatus::Loading | InstanceStatus::Online | InstanceStatus::Shutdown)) {
        return Err(ApiError::PortInUse);
    }
    let mut directory = std::env::current_dir()?;
    directory.push("saves");
    directory.push(name);
    let Some(mut directory) = directory.to_str() else {
        let mut out = String::new();
        out.push_str("Failed to convert path \"");
        out.push_str(directory.to_string_lossy().as_ref());
        out.push_str("\" to utf8");
        return Err(ApiError::IOError(out));
    };
    if directory.starts_with(r"\\?\") {
        directory = &directory[4..];
    }
    save::access(name)?;
    let mut child = match Command::new(get_java_path().as_str())
        .args(["-jar", "server.jar", "nogui"])
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Err(ApiError::JavaError(error.to_string())),
    };
    if child.stdin.is_none() && child.stdout.is_none() {
        return Err(ApiError::IOError("failed to capture process stdin and stdout".to_owned()))
    } else if child.stdin.is_none() {
        return Err(ApiError::IOError("failed to capture process stdin".to_owned()));
    } else if child.stdout.is_none() {
        return Err(ApiError::IOError("failed to capture process stdout".to_owned()));
    }
    let stdin = Arc::new(Mutex::new(child.stdin.take().unwrap()));
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let vector = Arc::new(InstanceVector::new());
    let instance = Instance {
        status: InstanceStatus::Loading,
        port,
        stdin,
        vector: vector.clone(),
    };
    let name_arc = Arc::new(name.to_owned());
    let name = name_arc.clone();
    // waits for child to complete
    tokio::spawn(async move {
        println!("[{name}] Java process spawned");
        match child.wait().await {
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
        if save::access(name.as_str()).is_err() {
            println!(
                "[!] An error occoured when attempting to set the access time of save \"{name}\""
            )
        }
        let mut instances = INSTANCES.write().await;
        if let Some(instance) = instances.get_mut(&*name) {
            instance.status = InstanceStatus::Offline;
            println!("[{name}] Waiter thread finished");
        } else {
            println!("[{name}] Waiter thread finished, and its instance was removed");
        }
    });
    let name: Arc<String> = name_arc.clone();
    // reads and parses stdout
    tokio::spawn(async move {
        println!("[{name}] Reader task spawned");
        let mut looking_for_done = true;
        let mut line = Vec::with_capacity(4 * 1024);
        let mut stdout = stdout;
        loop {
            line.clear();
            match stdout.read_until(b'\n', &mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    if line.is_empty() {
                        continue;
                    }
                    if looking_for_done && bytes_contains(line.as_slice(), b" Done ") {
                        looking_for_done = false;
                        let mut instances = INSTANCES.write().await;
                        if let Some(instance) = instances.get_mut(name.as_str()) {
                            if instance.status == InstanceStatus::Loading {
                                instance.status = InstanceStatus::Online;
                                if is_shutdown() {
                                    if instance.stop().await.is_err() {
                                        panic!("could not send stop command through stdin");
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    vector.write(&line).await;
                    println!("[{name}] {}", String::from_utf8_lossy(&line));
                }
                Err(error) => {
                    println!("[{name}] Error reading process stdout: {:?}", error);
                    break;
                }
            }
        }
        vector.finish().await;
        println!("[{name}] Reader task finished");
    });
    instances.insert(name_arc.to_string(), instance);
    Ok(())
}

/// stops the instance, returns immedialty, will return an error if it is not online
pub async fn stop_instance(name: &str) -> Result<(), ApiError> {
    save::exists(name)?;
    let mut instances = INSTANCES.write().await;
    if let Some(instance) = instances.get_mut(name) {
        instance.stop().await
    } else {
        Err(ApiError::BadInstanceStatus(InstanceStatus::Cold))
    }
}

/// checks if the instance is online, may returns an error if it is not online
pub async fn query_instance(name: &str) -> Result<InstanceStatus, ApiError> {
    save::exists(name)?;
    let instances = INSTANCES.read().await;
    if let Some(instance) = instances.get(name) {
        Ok(instance.status)
    } else {
        Ok(InstanceStatus::Cold)
    }
}

/// returns the instance vector, which can be subscribed to to receive read and wait for stdout
pub async fn read_instance(name: &str) -> Result<Arc<InstanceVector>, ApiError> {
    save::exists(name)?;
    let instances = INSTANCES.read().await;
    if let Some(instance) = instances.get(name) {
        Ok(instance.vector.clone())
    } else {
        Err(ApiError::BadInstanceStatus(InstanceStatus::Cold))
    }
}

/// writes to the stdin of the instance
pub async fn write_instance(name: &str, command: &str) -> Result<(), ApiError> {
    save::exists(name)?;
    if command.as_bytes().iter().any(|x| matches!(x, 0..=31 | 127)) {
        return Err(ApiError::BadRequest);
    }
    let command = command.trim();
    if command == "/stop" {
        return stop_instance(name).await;
    }
    let mut out = String::new();
    if command.starts_with('/') {
        out.push_str(&command[1..]);
    } else {
        out.push_str("say ");
        out.push_str(command);
    }
    out.push_str("\r\n");
    let instances = INSTANCES.read().await;
    if let Some(instance) = instances.get(name) {
        if instance.status != InstanceStatus::Online {
            Err(instance.status.to_error())
        } else if let Err(error) = instance
            .stdin
            .lock()
            .await
            .write(out.as_bytes())
            .await
        {
            Err(error.into())
        } else {
            Ok(())
        }
    } else {
        Err(ApiError::BadInstanceStatus(InstanceStatus::Cold))
    }
}

pub async fn instance_status_summary() -> String {
    let mut out = String::with_capacity(4 * 1024);
    out.push('{');
    append_comma_separated(
        INSTANCES.read().await.iter(),
        &mut out,
        |out, (name, instance)| {
            append_json_string(out, name);
            match instance.status {
                InstanceStatus::Cold => unreachable!(),
                InstanceStatus::Loading => *out += r#":"loading""#,
                InstanceStatus::Online => *out += r#":"online""#,
                InstanceStatus::Shutdown => *out += r#":"shutdown""#,
                InstanceStatus::Offline => *out += r#":"offline""#,
            }
        },
    );
    out.push('}');
    out
}

/// returns immediatly, signals to all instances that they must stop as soon as possible
pub async fn stop_all_instances() {
    println!("[*] Shutting down all instances");
    for (_, instance) in INSTANCES.write().await.iter_mut() {
        if instance.status == InstanceStatus::Online {
            if instance.stop().await.is_err() {
                panic!("could not send stop command through stdin");
            }
        }
        let _ = instance.vector.sender.send((Vec::new(), false));
    }
}

fn bytes_contains(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    if needle.is_empty() {
        return true;
    }
    for index in 0..=(haystack.len() - needle.len()) {
        if &haystack[index..index + needle.len()] == needle {
            return true;
        }
    }
    false
}

pub fn set_java_path(java: String) {
    let mut lock = JAVA_PATH.lock().expect("JAVA_PATH lock is poisoned");
    *lock = java;
}

pub fn get_java_path<'a>() -> std::sync::MutexGuard<'a, String> {
    JAVA_PATH.lock().expect("JAVA_PATH lock is poisoned")
}
