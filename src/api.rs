
use serde::{Deserialize, Serialize};
use crate::warp_utils::{WarpResult, catch};

// APIS //

#[derive(Deserialize, Serialize)]
pub struct CreateWorld {
    name: String,
    version: String,
}

impl CreateWorld {
    pub fn post(self) -> warp::reply::Html<String> {
        warp::reply::html(format!("<h1>Create World Response</h1><h2>name: {}</h2><h2>version: {}</h2>", self.name, self.version))
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
        Ok(versions) => WarpResult::Ok(warp::reply::json(&versions)),
        Err(_) => WarpResult::INTERNAL_SERVER_ERROR,
    }
}

pub fn saves() -> WarpResult<warp::reply::Json> {
    let saves: Result<Vec<String>, std::io::Error> = catch!({
        let mut saves = Vec::new();
        for path in std::fs::read_dir("saves")? {
            let path = path?;
            if matches!(path.metadata(), Ok(md) if md.is_dir()) {
                if let Some(filename) = path.file_name().to_str() {
                    saves.push(filename.to_owned())
                }
            }
        }
        Ok(saves)
    });

    match saves {
        Ok(saves) => WarpResult::Ok(warp::reply::json(&saves)),
        Err(_) => WarpResult::INTERNAL_SERVER_ERROR,
    }
}

pub fn icons(save: String) -> WarpResult<warp::reply::WithHeader<Vec<u8>>> {
    if !is_safe(&save) {
        return WarpResult::BAD_REQUEST;
    }
    const UNKNOWN_PNG: &[u8] = include_bytes!("../static/unknown.png");
    let data = match std::fs::read(format!("saves/{save}/world/icon.png")) {
        Ok(data) => data,
        Err(_) => UNKNOWN_PNG.to_owned(),
    };
    WarpResult::Ok(warp::reply::with_header(data, "Content-Type", "image/x-png"))
}

fn is_safe(text: &str) -> bool {
    if text.is_empty() || !text.is_ascii() || text.ends_with('.') {
        return false;
    }
    if text.starts_with(' ') || text.ends_with(' ') {
        return false;
    }
    for byte in text.as_bytes() {
        if matches!(*byte, 0..=31 | 127 | b'/' | b'\\' | b':' | b'*' | b'?' | b'"' | b'<' | b'>') {
            return false;
        }
    }
    const INVALID: [&str; 24] = [
        ".", "..",
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if INVALID.iter().any(|x| text.eq_ignore_ascii_case(x)) {
        return false;
    }
    true
}
