use crate::instances::InstanceStatus;
use crate::properties::*;
use crate::utils::{append_comma_separated, append_json_string, now, ApiError};
use std::collections::HashMap;
use std::io::Write;

pub async fn download_version(version: &str) -> Result<(), ApiError> {
    let path = format!("versions/{version}.jar");
    if std::fs::metadata(&path).is_ok_and(|x| x.is_file()) {
        return Ok(());
    }
    let client = reqwest::Client::new();
    let html = match async {
        client
            .get(format!("https://mcversions.net/download/{version}.html"))
            .send()
            .await?
            .text()
            .await
    }
    .await
    {
        Ok(html) => html,
        Err(error) => return Err(ApiError::IOError(error.to_string())),
    };
    let Some(link) = html
        .split("<a ")
        .skip(1)
        .filter_map(|version| {
            version
                .split_once(">Download Server Jar</a>")
                .and_then(|(attrs, _)| attrs.split_once("href=\""))
                .and_then(|(_, href)| href.split_once('"'))
                .map(|(link, _)| link)
        })
        .next()
    else {
        return Ok(());
    };

    let mut file = std::fs::File::options()
        .write(true)
        .create(true)
        .open(&path)?;

    async {
        let mut response = client
            .get(link)
            .send()
            .await
            .map_err(|x| ApiError::IOError(x.to_string()))?;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|x| ApiError::IOError(x.to_string()))?
        {
            file.write_all(&chunk)?;
        }
        Ok(())
    }
    .await
    .inspect_err(|_| {
        let _ = std::fs::remove_file(&path);
    })
}

/// an iterator to list all saves in the saves folder
///
/// instanciate with `Save::iter()`
pub struct SaveIter(Option<std::fs::ReadDir>);

pub mod save {
    use std::io::ErrorKind;

    use super::*;
    /// creates the save with the version specified and returns the same as load would
    pub fn create(
        name: &str,
        version: &str,
        values: HashMap<String, PropValue>,
    ) -> Result<String, ApiError> {
        match exists(name) {
            Err(ApiError::NotFound) => {}
            Err(error) => return Err(error),
            Ok(()) => return Err(ApiError::AlreadyExists),
        }
        if !std::fs::metadata(format!("versions/{version}.jar")).is_ok() {
            return Err(ApiError::VersionNotFound);
        }
        validate_properties(&values)?;
        std::fs::create_dir(format!("saves/{name}"))?;
        let properties = generate_properties(version, &values);
        if let Err(error) = (|| {
            // folder created successfully, move files into the folder
            std::fs::write(
                format!("saves/{name}/eula.txt"),
                "# file created by mc-manager\r\neula=true\r\n",
            )?;
            std::fs::write(format!("saves/{name}/server.properties"), properties)?;
            std::fs::copy(
                format!("versions/{version}.jar"),
                format!("saves/{name}/server.jar"),
            )?;
            Ok::<(), std::io::Error>(())
        })() {
            std::fs::remove_dir_all(format!("saves/{name}"))?;
            // the creation failed
            return Err(error.into());
        }
        load(name, InstanceStatus::Offline)
    }
    /// delete the save specified and all backups
    pub fn delete(name: &str) -> Result<(), ApiError> {
        std::fs::remove_dir_all(format!("saves/{name}"))?;
        Ok(())
    }
    /// returns a valid json with all of the properties for a save, including its name, and its status
    ///
    /// you must query status yourself, this is so the functions stays sync
    pub fn load(name: &str, status: InstanceStatus) -> Result<String, ApiError> {
        exists(name)?;
        let properties = read_properties(format!("saves/{name}/server.properties"))?;
        let mut out = String::with_capacity(4096);
        out += "{\"name\":\"";
        out += name;
        out += "\",\"status\":";
        match status {
            InstanceStatus::Cold => out += "\"cold\"",
            InstanceStatus::Loading => out += "\"loading\"",
            InstanceStatus::Online => out += "\"online\"",
            InstanceStatus::Shutdown => out += "\"shutdown\"",
            InstanceStatus::Offline => out += "\"offline\"",
        }
        for prop in PROPERTIES.iter() {
            if prop.access == PropAccess::None {
                continue;
            }
            out += ",\"";
            out += prop.name;
            out += "\":";
            if let Some(value) = properties.get(prop.name) {
                match prop.ty {
                    PropType::Bool(_)
                    | PropType::Int(..)
                    | PropType::Uint(..)
                    | PropType::IntEnum(..) => {
                        out += value;
                    }
                    PropType::String(_) | PropType::Datetime | PropType::StrEnum(..) => {
                        append_json_string(&mut out, &value)
                    }
                }
            } else {
                out += "null";
            }
        }
        out += "}";
        Ok(out)
    }
    /// modifies one property of the save
    pub fn modify(name: &str, values: HashMap<String, PropValue>) -> Result<(), ApiError> {
        exists(name)?;
        validate_properties(&values)?;
        write_properties(format!("saves/{name}/server.properties"), values)
    }
    /// update the access time of the world specified to now
    pub fn access(name: &str) -> Result<(), ApiError> {
        let mut values = HashMap::new();
        values.insert(
            "mc-manager-access-time".to_owned(),
            PropValue::String(now()),
        );
        write_properties(format!("saves/{name}/server.properties"), values)
    }
    /// returns a json in a string that describes all possible property values
    pub fn schema() -> String {
        let mut out = String::with_capacity(24 * 1024);
        out += r#"{"schema":{"#;
        append_comma_separated(PROPERTIES.iter(), &mut out, |out, prop| {
            if prop.access == PropAccess::None {
                return;
            }
            append_json_string(out, prop.name);
            *out += ":{";
            match prop.access {
                PropAccess::Write => *out += r#""access":"write""#,
                PropAccess::Read => *out += r#""access":"read""#,
                PropAccess::None => unreachable!(),
            }
            *out += ",\"type\":";
            match &prop.ty {
                PropType::Bool(true) => *out += r#"{"name":"boolean","default":true}"#,
                PropType::Bool(false) => *out += r#"{"name":"boolean","default":false}"#,
                PropType::String(value) => {
                    *out += r#"{"name":"string","default":"#;
                    append_json_string(out, value);
                    *out += "}";
                }
                PropType::Int(value, min, max) => {
                    *out += r#"{"name":"integer","default":"#;
                    *out += &value.to_string();
                    *out += r#","min":"#;
                    *out += &min.to_string();
                    *out += r#","max":"#;
                    *out += &max.to_string();
                    *out += "}";
                }
                PropType::Uint(value, min, max) => {
                    *out += r#"{"name":"integer","default":"#;
                    *out += &value.to_string();
                    *out += r#","min":"#;
                    *out += &min.to_string();
                    *out += r#","max":"#;
                    *out += &max.to_string();
                    *out += "}";
                }
                PropType::Datetime => *out += r#"{"name":"string","default":""}"#,
                PropType::IntEnum(value, members) => {
                    *out += r#"{"name":"integer-enum","default":"#;
                    *out += &value.to_string();
                    *out += r#","members":["#;
                    append_comma_separated(members.iter().map(|x| *x), out, append_json_string);
                    *out += "]}";
                }
                PropType::StrEnum(value, members) => {
                    *out += r#"{"name":"string-enum","default":"#;
                    *out += &value.to_string();
                    *out += r#","members":["#;
                    append_comma_separated(members.iter(), out, |out, member| {
                        *out += "[";
                        append_json_string(out, member.0);
                        *out += ",";
                        append_json_string(out, member.1);
                        *out += "]";
                    });
                    *out += "]}";
                }
            }
            *out += r#","label":"#;
            append_json_string(out, prop.label);
            *out += r#","desc":"#;
            append_json_string(out, prop.desc);
            *out += "}";
        });
        out += r#"},"create_properties":["#;
        append_comma_separated(
            CREATE_PROPERTIES.iter().map(|x| *x),
            &mut out,
            append_json_string,
        );
        out += "]}";
        out
    }
    /// iterate over the names of all saves avaiable
    pub fn iter() -> Result<SaveIter, ApiError> {
        match std::fs::read_dir("saves") {
            Ok(paths) => Ok(SaveIter(Some(paths))),
            Err(error) => Err(error.into()),
        }
    }
    /// returns the apropiate error if the save does not exist, may return IOError
    pub fn exists(name: &str) -> Result<(), ApiError> {
        match std::fs::metadata(format!("saves/{name}")) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    Ok(())
                } else {
                    Err(ApiError::NotFound)
                }
            }
            Err(error) => {
                if error.kind() == ErrorKind::NotFound {
                    Err(ApiError::NotFound)
                } else {
                    Err(error.into())
                }
            }
        }
    }
}

impl Iterator for SaveIter {
    type Item = Result<String, ApiError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.0 {
            match iter.next() {
                Some(Ok(path)) => {
                    if let Some(filename) = path.file_name().to_str() {
                        Some(Ok(filename.to_owned()))
                    } else {
                        self.next()
                    }
                }
                Some(Err(error)) => {
                    self.0 = None;
                    Some(Err(error.into()))
                }
                None => {
                    self.0 = None;
                    None
                }
            }
        } else {
            None
        }
    }
}
