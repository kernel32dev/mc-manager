use std::collections::HashMap;
use std::convert::Infallible;

use crate::instances::*;
use crate::properties::PropValue;
use crate::server::is_shutdown;
use crate::state::save;
use crate::utils::{catch, json_response, WarpResult, SaveError};
use serde::Deserialize;
use warp::reply::Response;
use warp::Reply;

// APIS //

#[derive(Deserialize)]
pub struct CreateSave {
    name: String,
    version: String,
    values: HashMap<String, PropValue>,
}

pub fn create_save(body: CreateSave) -> WarpResult<Response> {
    save::create(&body.name, &body.version, body.values)
        .map(json_response)
        .into()
}

#[derive(Deserialize)]
pub struct ModifySave {
    name: String,
    values: HashMap<String, PropValue>,
}

pub async fn modify_save(body: ModifySave) -> Result<WarpResult<Response>, Infallible> {
    match query_instance(&body.name).await {
        Ok(InstanceStatus::Offline) => Ok(save::modify(&body.name, body.values).into()),
        Ok(status) => Ok(WarpResult::Err(status.to_error())),
        Err(error) => Ok(WarpResult::Err(error)),
    }
}

#[derive(Deserialize)]
pub struct DeleteSave {
    name: String,
}

pub async fn delete_save(body: DeleteSave) -> Result<WarpResult<Response>, Infallible> {
    match query_instance(&body.name).await {
        Ok(InstanceStatus::Offline) => Ok(save::delete(&body.name).into()),
        Ok(status) => Ok(WarpResult::Err(status.to_error())),
        Err(error) => Ok(WarpResult::Err(error)),
    }
}

#[derive(Deserialize)]
pub struct StartSave {
    name: String,
}

pub async fn start_save(body: StartSave) -> Result<WarpResult<Response>, Infallible> {
    Ok(start_instance(&body.name).await.into())
}

#[derive(Deserialize)]
pub struct StopSave {
    name: String,
}

pub async fn stop_save(body: StopSave) -> Result<WarpResult<Response>, Infallible> {
    Ok(stop_instance(&body.name).await.into())
}

pub fn versions() -> WarpResult<warp::reply::Json> {
    let versions: Result<Vec<String>, std::io::Error> = catch!({
        let mut versions = Vec::new();
        for path in std::fs::read_dir("versions")? {
            let path = path?;
            if let Some(filename) = path.file_name().to_str() {
                if matches!(path.metadata(), Ok(md) if md.is_file()) {
                    if let Some(name) = filename.strip_suffix(".jar") {
                        versions.push(name.to_owned())
                    }
                }
            }
        }
        Ok(versions)
    });

    match versions {
        Ok(versions) => Ok(warp::reply::json(&versions)),
        Err(_) => Err(SaveError::IOError),
    }.into()
}

pub async fn saves() -> Result<WarpResult<Response>, Infallible> {
    Ok((|| async {
        let mut body = String::with_capacity(16 * 1024);
        body.push_str("{\"saves\":[");
        for name in save::iter()? {
            let name = name?;
            body.push_str(&save::load(&name, query_instance(&name).await?)?);
            body.push(',');
        }
        match body.pop() {
            Some('[') => body.push_str("[]"),
            Some(',') => body.push(']'),
            _ => unreachable!(),
        }
        body.push('}');
        Ok(json_response(body))
    })().await.into())
}

pub fn icons(save: String) -> WarpResult<warp::reply::WithHeader<Vec<u8>>> {
    if !is_safe(&save) {
        return WarpResult::Err(SaveError::BadRequest);
    }
    const UNKNOWN_PNG: &[u8] = include_bytes!("../static/assets/unknown.png");
    let data = match std::fs::read(format!("saves/{save}/world/icon.png")) {
        Ok(data) => data,
        Err(_) => UNKNOWN_PNG.to_owned(),
    };
    WarpResult::Ok(warp::reply::with_header(
        data,
        "Content-Type",
        "image/x-png",
    ))
}

pub fn schema() -> WarpResult<Response> {
    WarpResult::Ok(json_response(save::schema()))
}

pub async fn status() -> Result<WarpResult<Response>, Infallible> {
    Ok(WarpResult::Ok(json_response(instance_status_summary().await)))
}

#[derive(Deserialize)]
pub struct Command {
    name: String,
    command: String,
}

pub async fn command(body: Command) -> Result<WarpResult<Response>, Infallible> {
    //use futures::FutureExt;
    Ok(write_instance(&body.name, &body.command).await.into())
}

pub async fn console(mut offset: usize, save: String, ws: warp::ws::Ws) -> Result<WarpResult<impl Reply>, Infallible> {
    //use futures::FutureExt;
    match read_instance(&save).await {
        Ok(vector) => Ok(WarpResult::Ok(ws.on_upgrade(move |mut ws| async move {
            println!("[*] Websocket stream spawned");
            use futures::SinkExt;
            let mut subscription = vector.subscribe();
            while !is_shutdown() {
                let pair = {
                    let borrow = subscription.borrow();
                    let data = &borrow.0;
                    let alive = &borrow.1;
                    if offset >= data.len() {
                        if !*alive {
                            println!("[*] Websocket stream finished");
                            break;
                        }
                        None
                    } else {
                        Some((data[offset..].to_owned(), data.len()))
                    }
                };
                if let Some((payload, new_offset)) = pair {
                    if ws.send(warp::ws::Message::binary(payload)).await.is_err() {
                        println!("[*] Websocket stream finished, due to error");
                        break;
                    }
                    println!("[*] Websocket sent {} bytes", new_offset - offset);
                    offset = new_offset;
                }
                if subscription.changed().await.is_err() {
                    println!("[*] Websocket stream finished, because sender half was dropped");
                    break;
                }
            }
            let _ = ws.close().await;
        }))),
        Err(error) => Ok(WarpResult::Err(error)),
    }
}

fn is_safe(text: &str) -> bool {
    if text.is_empty() || !text.is_ascii() || text.ends_with('.') {
        return false;
    }
    if text.starts_with(' ') || text.ends_with(' ') {
        return false;
    }
    for byte in text.as_bytes() {
        if matches!(
            *byte,
            0..=31 | 127 | b'/' | b'\\' | b':' | b'*' | b'?' | b'"' | b'<' | b'>'
        ) {
            return false;
        }
    }
    const INVALID: [&str; 24] = [
        ".", "..", "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
        "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9",
    ];
    if INVALID.iter().any(|x| text.eq_ignore_ascii_case(x)) {
        return false;
    }
    true
}
