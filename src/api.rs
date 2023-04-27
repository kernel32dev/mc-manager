
use std::collections::HashMap;

use crate::state::{save, SaveError, PropValue};
use crate::warp_utils::{catch, WarpResult};
use serde::Deserialize;
use warp::Reply;
use warp::reply::Response;

// APIS //

#[derive(Deserialize)]
pub struct CreateSave {
    name: String,
    version: String,
    values: HashMap<String, PropValue>,
}

impl CreateSave {
    pub fn post(self) -> WarpResult<Response> {
        save::create(&self.name, &self.version, self.values).map(json_response).into()
    }
}

#[derive(Deserialize)]
pub struct ModifySave {
    name: String,
    values: HashMap<String, PropValue>,
}

impl ModifySave {
    pub fn post(self) -> WarpResult<Response> {
        save::modify(&self.name, self.values).map(|_| warp::reply().into_response()).into()
    }
}

#[derive(Deserialize)]
pub struct DeleteSave {
    name: String,
}

impl DeleteSave {
    pub fn post(self) -> WarpResult<Response> {
        save::delete(&self.name).map(|_| warp::reply().into_response()).into()
    }
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
        Ok(versions) => WarpResult::Reply(warp::reply::json(&versions)),
        Err(_) => WarpResult::INTERNAL_SERVER_ERROR,
    }
}

pub fn saves() -> WarpResult<Response> {
    let result: Result<Response, SaveError> = catch!({
        let mut body = String::with_capacity(16 * 1024);
        body.push_str("{\"saves\":[");
        for name in save::iter()? {
            body.push_str(&save::load(&name?)?);
            body.push(',');
        }
        match body.pop() {
            Some('[') => body.push_str("[]"),
            Some(',') => body.push(']'),
            _ => unreachable!(),
        }
        body.push('}');
        Ok(json_response(body))
    });
    result.into()
}

pub fn icons(save: String) -> WarpResult<warp::reply::WithHeader<Vec<u8>>> {
    if !is_safe(&save) {
        return WarpResult::BAD_REQUEST;
    }
    const UNKNOWN_PNG: &[u8] = include_bytes!("../static/assets/unknown.png");
    let data = match std::fs::read(format!("saves/{save}/world/icon.png")) {
        Ok(data) => data,
        Err(_) => UNKNOWN_PNG.to_owned(),
    };
    WarpResult::Reply(warp::reply::with_header(
        data,
        "Content-Type",
        "image/x-png",
    ))
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

/// creates a json reponse from raw body with the appropiate content-type
fn json_response(body: String) -> Response {
    use warp::http::header::CONTENT_TYPE;
    let (mut parts, body) = Response::new(body.into()).into_parts();
    parts.headers.append(CONTENT_TYPE, "application/json".parse().unwrap());
    Response::from_parts(parts, body)
}
