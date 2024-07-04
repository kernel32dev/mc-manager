use std::collections::HashMap;
use std::convert::Infallible;

use crate::properties::PropValue;
use crate::server::is_shutdown;
use crate::state::save;
use crate::utils::{json_response, ApiError, WarpResult};
use crate::{instances::*, state};
use serde::Deserialize;
use warp::Reply;

static VERSION_CACHE: std::sync::RwLock<Option<&'static str>> = std::sync::RwLock::new(None);

// APIS //

#[derive(Deserialize)]
pub struct CreateSave {
    name: String,
    version: String,
    values: HashMap<String, PropValue>,
}

pub async fn create_save(body: CreateSave) -> Result<WarpResult<impl Reply>, Infallible> {
    if !is_safe(&body.name) {
        return Ok(WarpResult::Err(ApiError::BadName));
    }
    if let Err(error) = state::download_version(&body.version).await {
        return Ok(WarpResult::Err(error.into()));
    }
    Ok(save::create(&body.name, &body.version, body.values)
        .map(json_response)
        .into())
}

#[derive(Deserialize)]
pub struct ModifySave {
    name: String,
    values: HashMap<String, PropValue>,
}

pub async fn modify_save(body: ModifySave) -> Result<WarpResult<impl Reply>, Infallible> {
    if !is_safe(&body.name) {
        return Ok(WarpResult::Err(ApiError::BadName));
    }
    match query_instance(&body.name).await {
        Ok(InstanceStatus::Offline | InstanceStatus::Cold) => {
            Ok(save::modify(&body.name, body.values).into())
        }
        Ok(status) => Ok(WarpResult::Err(status.to_error())),
        Err(error) => Ok(WarpResult::Err(error)),
    }
}

#[derive(Deserialize)]
pub struct DeleteSave {
    name: String,
}

pub async fn delete_save(body: DeleteSave) -> Result<WarpResult<impl Reply>, Infallible> {
    if !is_safe(&body.name) {
        return Ok(WarpResult::Err(ApiError::BadName));
    }
    match query_instance(&body.name).await {
        Ok(InstanceStatus::Offline | InstanceStatus::Cold) => Ok(save::delete(&body.name).into()),
        Ok(status) => Ok(WarpResult::Err(status.to_error())),
        Err(error) => Ok(WarpResult::Err(error)),
    }
}

#[derive(Deserialize)]
pub struct StartSave {
    name: String,
}

pub async fn start_save(body: StartSave) -> Result<WarpResult<impl Reply>, Infallible> {
    if !is_safe(&body.name) {
        return Ok(WarpResult::Err(ApiError::BadName));
    }
    Ok(start_instance(&body.name).await.into())
}

#[derive(Deserialize)]
pub struct StopSave {
    name: String,
}

pub async fn stop_save(body: StopSave) -> Result<WarpResult<impl Reply>, Infallible> {
    if !is_safe(&body.name) {
        return Ok(WarpResult::Err(ApiError::BadName));
    }
    Ok(stop_instance(&body.name).await.into())
}

pub async fn versions() -> Result<WarpResult<impl Reply>, Infallible> {
    if let Some(versions) = *VERSION_CACHE.read().unwrap() {
        return Ok(WarpResult::Ok(json_response(versions)));
    }
    let response = match async {
        reqwest::Client::new()
            .get("https://mcversions.net")
            .send()
            .await?
            .text()
            .await
    }
    .await
    {
        Ok(response) => response,
        Err(error) => return Ok(WarpResult::Err(ApiError::IOError(error.to_string()))),
    };

    let existing_versions: Result<Vec<String>, std::io::Error> = (|| {
        let mut versions = Vec::new();
        for path in std::fs::read_dir("versions")? {
            let path = path?;
            if let Some(filename) = path.file_name().to_str() {
                if matches!(path.metadata(), Ok(md) if md.is_file()) {
                    if let Some(name) = filename.strip_suffix(".jar") {
                        versions.push(name.to_owned());
                    }
                }
            }
        }
        Ok(versions)
    })();
    let mut existing_versions = match existing_versions {
        Ok(versions) => versions,
        Err(error) => return Ok(WarpResult::Err(error.into())),
    };

    let mut versions = String::new();
    let mut online_versions = String::new();
    for version in response.split("data-version=\"").skip(1) {
        if !version.starts_with(|x: char| x.is_ascii_digit()) {
            continue;
        }
        let Some((version, _)) = version.split_once('"') else {
            continue;
        };
        if let Some(index) = existing_versions.iter().position(|x| x == version) {
            existing_versions.remove(index);
        }
        online_versions.push_str("\"");
        online_versions.push_str(version);
        online_versions.push_str("\",");
    }
    versions.push('[');
    for i in &existing_versions {
        online_versions.push_str("\"");
        online_versions.push_str(&i);
        online_versions.push_str("\",");
    }
    versions.push_str(&online_versions);
    if versions.ends_with(',') {
        versions.pop();
    }
    versions.push(']');

    let versions = *VERSION_CACHE
        .write()
        .unwrap()
        .get_or_insert_with(|| versions.leak());

    Ok(WarpResult::Ok(json_response(versions)))
}

/*
pub fn versions() -> WarpResult<impl Reply> {
    let versions: Result<Vec<String>, std::io::Error> = (||{
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
    })();
    match versions {
        Ok(versions) => Ok(warp::reply::json(&versions)),
        Err(error) => Err(error.into()),
    }.into()
}
*/

pub async fn saves() -> Result<WarpResult<impl Reply>, Infallible> {
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
    })()
    .await
    .into())
}

pub fn icons(save: String) -> WarpResult<impl Reply> {
    let save = match parse_name(save) {
        Ok(save) => save,
        Err(error) => return WarpResult::Err(error),
    };
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

pub fn schema() -> WarpResult<impl Reply> {
    WarpResult::Ok(json_response(save::schema()))
}

pub async fn status() -> Result<WarpResult<impl Reply>, Infallible> {
    Ok(WarpResult::Ok(json_response(
        instance_status_summary().await,
    )))
}

#[derive(Deserialize)]
pub struct Command {
    name: String,
    command: String,
}

pub async fn command(body: Command) -> Result<WarpResult<impl Reply>, Infallible> {
    if !is_safe(&body.name) {
        return Ok(WarpResult::Err(ApiError::BadName));
    }
    Ok(write_instance(&body.name, &body.command).await.into())
}

pub async fn console(
    mut offset: usize,
    save: String,
    ws: warp::ws::Ws,
) -> Result<WarpResult<impl Reply>, Infallible> {
    #[cfg(debug_assertions)]
    const DEBUG_WEB_SOCKET: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG_WEB_SOCKET: bool = false;
    let save = match parse_name(save) {
        Ok(save) => save,
        Err(error) => return Ok(WarpResult::Err(error)),
    };
    if DEBUG_WEB_SOCKET {
        println!("[*] Websocket: reading console of {}", &save);
    }
    match read_instance(&save).await {
        Ok(vector) => Ok(WarpResult::Ok(ws.on_upgrade(move |mut ws| async move {
            if DEBUG_WEB_SOCKET {
                println!("[*] Websocket stream spawned");
            }
            use futures::SinkExt;
            let mut subscription = vector.subscribe();
            while !is_shutdown() {
                let pair = {
                    let borrow = subscription.borrow();
                    let data = &borrow.0;
                    let alive = &borrow.1;
                    if offset >= data.len() {
                        if !*alive {
                            if DEBUG_WEB_SOCKET {
                                println!("[*] Websocket stream finished");
                            }
                            break;
                        }
                        None
                    } else {
                        Some((data[offset..].to_owned(), data.len()))
                    }
                };
                if let Some((payload, new_offset)) = pair {
                    if ws.send(warp::ws::Message::binary(payload)).await.is_err() {
                        if DEBUG_WEB_SOCKET {
                            println!("[*] Websocket stream finished, due to error");
                        }
                        break;
                    }
                    if DEBUG_WEB_SOCKET {
                        println!("[*] Websocket sent {} bytes", new_offset - offset);
                    }
                    offset = new_offset;
                }
                if subscription.changed().await.is_err() {
                    if DEBUG_WEB_SOCKET {
                        println!("[*] Websocket stream finished, because sender half was dropped");
                    }
                    break;
                }
            }
            let _ = ws.close().await;
        }))),
        Err(error) => Ok(WarpResult::Err(error)),
    }
}

fn parse_name(name: String) -> Result<String, ApiError> {
    fn from_hex_byte(char: u8) -> Option<u8> {
        match char {
            b'a'..=b'f' => Some(char - b'a' + 10),
            b'A'..=b'F' => Some(char - b'A' + 10),
            b'0'..=b'9' => Some(char - b'0'),
            _ => None,
        }
    }
    fn from_uri_encoded(name: String) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(name.len());
        let mut iter = name.bytes();
        while let Some(byte) = iter.next() {
            match byte {
                b'%' => {
                    let upper = iter.next()?;
                    let lower = iter.next()?;
                    let byte = (from_hex_byte(upper)? * 0x10) | from_hex_byte(lower)?;
                    if byte >= 0x80 {
                        return None;
                    }
                    out.push(byte);
                }
                0x00..=0x7F => out.push(byte),
                0x80..=0xFF => return None,
            }
        }
        Some(out)
    }
    if name.contains('%') {
        let Some(decoded) = from_uri_encoded(name) else {
            return Err(ApiError::BadRequest);
        };
        let Ok(text) = std::str::from_utf8(&decoded) else {
            return Err(ApiError::BadRequest);
        };
        if is_safe(text) {
            Ok(text.to_owned())
        } else {
            Err(ApiError::BadName)
        }
    } else {
        if is_safe(&name) {
            Ok(name)
        } else {
            Err(ApiError::BadName)
        }
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
