
use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use warp::Filter;
use crate::api::*;
use crate::instances::stop_all_instances;
use crate::properties::read_properties;
use crate::utils::filters;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

const CONFIG_FILE: &str = "mc-manager.properties";
const DEFAULT_CONFIG_FILE: &str = "#mc-manager configurations file\r\n\r\nip=\r\nport=1234\r\n";

pub fn is_shutdown() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

fn set_shutdown() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

pub fn serve(shutdown: Option<tokio::sync::oneshot::Receiver<()>>) {

    let apis = filters!(
        GET fn versions;
        GET async fn saves;
        GET fn icons String;
        GET fn schema;
        GET async fn status;
        WS async fn console usize String;
        POST fn create_save;
        POST async fn modify_save;
        POST async fn delete_save;
        POST async fn start_save;
        POST async fn stop_save;
        POST async fn command;
    );

    #[cfg(not(debug_assertions))] // cd into folder of executable
    std::env::set_current_dir(std::env::current_exe().expect("current_exe").parent().expect("parent")).expect("set_current_dir");

    #[cfg(not(debug_assertions))] // load assets from executable
    let routes = apis.or(static_dir::static_dir!("static"));

    #[cfg(debug_assertions)] // load assets from static directory
    let routes = apis.or(warp::fs::dir("static"));

    let config = match std::fs::metadata(CONFIG_FILE) {
        Ok(metadata) => {
            metadata.is_file()
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            std::fs::write(CONFIG_FILE, DEFAULT_CONFIG_FILE).is_ok()
        }
        Err(_) => false
    };
    let saves = match std::fs::metadata("saves") {
        Ok(metadata) => {
            metadata.is_dir()
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            std::fs::create_dir("saves").is_ok()
        }
        Err(_) => false
    };
    let versions = match std::fs::metadata("versions") {
        Ok(metadata) => {
            metadata.is_dir()
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            std::fs::create_dir("versions").is_ok()
        }
        Err(_) => false
    };

    if !config {
        println!("[!] ERROR: default \"{}\" file was not found and could not be created", CONFIG_FILE);
    }
    if !saves {
        println!("[!] ERROR: saves foulder was not found and could not be created");
    }
    if !versions {
        println!("[!] ERROR: versions foulder was not found and could not be created");
    }
    if !config || !saves || !versions {
        return;
    }

    let config = match read_properties(CONFIG_FILE) {
        Ok(config) => config,
        Err(_) => {
            println!("[!] ERROR: could not read config file");
            return;
        }
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("failed to build runtime");

    let (ip, port) = {
        let ip = config.get("ip");
        let port = config.get("port");
        if let (Some(ip), Some(port)) = (ip, port) {
            let ip = parse_ip(ip.trim());
            let port = parse_port(port.trim());
            if let (Some(ip), Some(port)) = (ip, port) {
                (ip, port)
            } else {
                if ip.is_none() {
                    println!("[!] ERROR: property ip is invalid");
                }
                if port.is_none() {
                    println!("[!] ERROR: property port is invalid");
                }
                return;
            }
        } else {
            if ip.is_none() {
                println!("[!] ERROR: property ip was not found");
            }
            if port.is_none() {
                println!("[!] ERROR: property port was not found");
            }
            return;
        }
    };

    if ip == [0, 0, 0, 0] {
        println!("[*] Minecraft Server Manager *:{}", port);
    } else {
        println!("[*] Minecraft Server Manager {}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port);
    }

    let _enter = rt.enter();
    rt.block_on(
        warp::serve(routes).bind_with_graceful_shutdown((ip, port), async move {
            if let Some(shutdown) = shutdown {
                shutdown.await.unwrap();
                println!("[*] Stopping service");
            } else {
                tokio::signal::ctrl_c().await.unwrap();
                println!("[*] CTRL-C detected");
            }
            set_shutdown();
            stop_all_instances().await;
        }).1
    );
}

fn parse_ip(ip: &str) -> Option<[u8; 4]> {
    if ip.is_empty() {
        return Some([0, 0, 0, 0]);
    }
    let parts: Vec<&str> = ip.splitn(5, '.').collect();
    if parts.len() != 4 {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
        parts[3].parse().ok()?,
    ])
}

fn parse_port(port: &str) -> Option<u16> {
    port.parse().ok()
}
