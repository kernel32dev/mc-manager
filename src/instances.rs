use crate::state::{read_property, save};
use crate::state::{save::exists};
use crate::utils::{append_comma_separated, append_json_string, SaveError};
use crate::server::is_shutdown;
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::process::{ChildStdin, Command};
use std::process::Stdio;
use tokio::sync::{Mutex, RwLock, watch};
use std::sync::Arc;

lazy_static! {
    static ref INSTANCES: RwLock<HashMap<String, Instance>> = RwLock::new(HashMap::new());
}

struct Instance {
    status: InstanceStatus,
    port: u16,
    stdin: Arc<Mutex<ChildStdin>>,
    vector: Arc<InstanceVector>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Offline,
    Loading,
    Online,
    Shutdown,
}

pub struct InstanceVector {
    sender: watch::Sender<(Vec<u8>, bool)>,
}

impl Instance {
    async fn stop(&mut self) -> Result<(), SaveError> {
        match self.status {
            InstanceStatus::Offline => unreachable!(),
            InstanceStatus::Loading => Err(SaveError::IsLoading),
            InstanceStatus::Online => {
                let mut stdin = self.stdin.lock().await;
                stdin.write(b"stop\r\n").await.unwrap();
                self.status = InstanceStatus::Shutdown;
                Ok(())
            }
            InstanceStatus::Shutdown => Ok(()),
        }
    }
}

impl InstanceStatus {
    pub fn to_error(self) -> SaveError {
        match self {
            InstanceStatus::Offline => SaveError::IsOffline,
            InstanceStatus::Loading => SaveError::IsLoading,
            InstanceStatus::Online => SaveError::IsOnline,
            InstanceStatus::Shutdown => SaveError::IsShutdown,
        }
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
        self.sender.send_modify(|(buffer, _)| buffer.extend_from_slice(data));
    }
    pub fn subscribe(&self) -> watch::Receiver<(Vec<u8>, bool)> {
        self.sender.subscribe()
    }
}

/// creates the instance, returns an error if it is already online
pub async fn start_instance(name: &str) -> Result<(), SaveError> {
    exists(name)?;
    let port = match read_property(format!("saves/{name}/server.properties"), "server-port")? {
        Some(port) => match port.parse() {
            Ok(port) => port,
            Err(_) => return Err(SaveError::IOError),
        },
        None => return Err(SaveError::IOError),
    };
    let mut instances = INSTANCES.write().await;
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
    save::access(name)?;
    let mut child = Command::new("java.exe")
        .args(["-jar", "server.jar", "nogui"])
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn java.exe");
    println!("[{name}] Java process spawned");
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
        println!("[{name}] Waiter thread spawned");
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
            println!("[!] An error occoured when attempting to set the access time of save \"{name}\"")
        }
        let mut instances = INSTANCES.write().await;
        instances
            .remove(&*name)
            .expect("Waiter thread finished, and its instance was removed");
        println!("[{name}] Waiter thread finished");
    });
    let name: Arc<String> = name_arc.clone();
    // reads and parses stdout
    tokio::spawn(async move {
        println!("[{name}] Reader thread spawned");
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
        println!("[{name}] Reader thread finished");
    });
    instances.insert(name_arc.to_string(), instance);
    Ok(())
}

/// stops the instance, returns immedialty, will return an error if it is not online
pub async fn stop_instance(name: &str) -> Result<(), SaveError> {
    exists(name)?;
    let mut instances = INSTANCES.write().await;
    if let Some(instance) = instances.get_mut(name) {
        instance.stop().await
    } else {
        Err(SaveError::IsOffline)
    }
}

/// checks if the instance is online, may returns an error if it is not online
pub async fn query_instance(name: &str) -> Result<InstanceStatus, SaveError> {
    exists(name)?;
    let instances = INSTANCES.read().await;
    if let Some(instance) = instances.get(name) {
        Ok(instance.status)
    } else {
        Ok(InstanceStatus::Offline)
    }
}

/// returns a stream to the stdout of the stream, optionally you can skip some bytes of the output
pub async fn read_instance(name: &str) -> Result<Arc<InstanceVector>, SaveError> {
    exists(name)?;
    let instances = INSTANCES.read().await;
    if let Some(instance) = instances.get(name) {
        Ok(instance.vector.clone())
    } else {
        Err(SaveError::IsOffline)
    }
}

/// returns a stream to the stdout of the stream, optionally you can skip some bytes of the output
pub async fn write_instance(name: &str, command: &str) -> Result<(), SaveError> {
    exists(name)?;
    if command.as_bytes().iter().any(|x| matches!(x, 0..=31 | 127)) {
        return Err(SaveError::BadRequest);
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
        } else if instance.stdin.lock().await.write(out.as_bytes()).await.is_ok() {
            Ok(())
        } else {
            Err(SaveError::IOError)
        }
    } else {
        Err(SaveError::IsOffline)
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
    if haystack.len() < needle.len() { return false; }
    if needle.is_empty() { return true; }
    for index in 0..=(haystack.len() - needle.len()) {
        if &haystack[index..index + needle.len()] == needle {
            return true;
        }
    }
    false
}
