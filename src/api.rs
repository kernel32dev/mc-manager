
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
