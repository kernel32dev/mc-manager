
use crate::properties::*;
use crate::instances::InstanceStatus;
use crate::utils::{now, append_json_string, append_comma_separated};
use std::collections::HashMap;
use std::io::ErrorKind;

/// an iterator to list all saves in the saves folder
///
/// instanciate with `Save::iter()`
pub struct SaveIter(Option<std::fs::ReadDir>);

use crate::utils::SaveError;

impl From<std::io::Error> for SaveError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            ErrorKind::AlreadyExists => Self::AlreadyExists,
            ErrorKind::NotFound => Self::NotFound,
            _ => Self::IOError,
        }
    }
}

pub mod save {
    use super::*;
    /// creates the save with the version specified and returns the same as load would
    pub fn create(
        name: &str,
        version: &str,
        values: HashMap<String, PropValue>,
    ) -> Result<String, SaveError> {
        if !std::fs::metadata(format!("versions/{version}.jar")).is_ok() {
            return Err(SaveError::VersionNotFound);
        }
        validate_properties(&values, PropAccess::Write)?;
        std::fs::create_dir(format!("saves/{name}"))?;
        let properties = generate_properties(version, &values);
        if (|| {
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
        })()
        .is_err()
        {
            std::fs::remove_dir_all(format!("saves/{name}"))?;
            // the creation failed
            return Err(SaveError::IOError);
        }
        load(name, InstanceStatus::Offline)
    }
    /// delete the save specified and all backups
    pub fn delete(name: &str) -> Result<(), SaveError> {
        std::fs::remove_dir_all(format!("saves/{name}"))?;
        Ok(())
    }
    /// returns a valid json with all of the properties for a save, including its name, and its status
    ///
    /// you must query status yourself, this is so the functions stays sync
    pub fn load(name: &str, status: InstanceStatus) -> Result<String, SaveError> {
        exists(name)?;
        let properties = read_properties(format!("saves/{name}/server.properties"))?;
        let mut out = String::with_capacity(4096);
        out += "{\"name\":\"";
        out += name;
        out += "\",\"status\":";
        match status {
            InstanceStatus::Offline => out += "\"offline\"",
            InstanceStatus::Loading => out += "\"loading\"",
            InstanceStatus::Online => out += "\"online\"",
            InstanceStatus::Shutdown => out += "\"shutdown\"",
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
    pub fn modify(name: &str, values: HashMap<String, PropValue>) -> Result<(), SaveError> {
        exists(name)?;
        validate_properties(&values, PropAccess::Write)?;
        write_properties(format!("saves/{name}/server.properties"), values)
    }
    /// update the access time of the world specified to now
    pub fn access(name: &str) -> Result<(), SaveError> {
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
        append_comma_separated(PROPERTIES.iter(), &mut out, |out, prop|{
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
        append_comma_separated(CREATE_PROPERTIES.iter().map(|x| *x), &mut out, append_json_string);
        out += "]}";
        out
    }
    /// iterate over the names of all saves avaiable
    pub fn iter() -> Result<SaveIter, SaveError> {
        match std::fs::read_dir("saves") {
            Ok(paths) => Ok(SaveIter(Some(paths))),
            Err(_) => Err(SaveError::IOError),
        }
    }
    /// returns an error if the save does not exist, may return IOError
    pub fn exists(name: &str) -> Result<(), SaveError> {
        match std::fs::metadata(format!("saves/{name}")) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    Ok(())
                } else {
                    Err(SaveError::NotFound)
                }
            }
            Err(error) => {
                const ERROR_FILE_NOT_FOUND: i32 = 2;
                const ERROR_PATH_NOT_FOUND: i32 = 3;
                if matches!(
                    error.raw_os_error(),
                    Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND)
                ) {
                    Err(SaveError::NotFound)
                } else {
                    Err(SaveError::IOError)
                }
            }
        }
    }
}

impl Iterator for SaveIter {
    type Item = Result<String, SaveError>;

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
                Some(Err(_)) => {
                    self.0 = None;
                    Some(Err(SaveError::IOError))
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
