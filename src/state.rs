
use crate::properties::*;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

/// an iterator to list all saves in the saves folder
///
/// instanciate with `Save::iter()`
pub struct SaveIter(Option<std::fs::ReadDir>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    NotFound,
    AlreadyExists,
    VersionNotFound,
    PropertyNotFound,
    IOError,
}

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
        load(name)
    }
    /// delete the save specified and all backups
    pub fn delete(name: &str) -> Result<(), SaveError> {
        std::fs::remove_dir_all(format!("saves/{name}"))?;
        Ok(())
    }
    /// returns a valid json with all of the properties for a save, including its name
    pub fn load(name: &str) -> Result<String, SaveError> {
        folder_exists(format!("saves/{name}"))?;
        let properties = read_properties(format!("saves/{name}/server.properties"))?;
        let mut out = String::with_capacity(4096);
        out += "{\"name\":\"";
        out += name;
        out += "\"";
        for prop in PROPERTIES.iter() {
            if prop.access == PropAccess::None {
                continue;
            }
            out += ",\"";
            out += name;
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
        folder_exists(format!("saves/{name}"))?;
        validate_properties(&values, PropAccess::Write)?;
        write_properties(format!("saves/{name}/server.properties"), values)
    }
    /// update the access time of the world specified to now
    #[allow(dead_code)]
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
        for prop in PROPERTIES.iter() {
            if prop.access == PropAccess::None {
                continue;
            }
            append_json_string(&mut out, prop.name);
            out += ":{";
            match prop.access {
                PropAccess::Write => out += r#""access":"write""#,
                PropAccess::Read => out += r#""access":"read""#,
                PropAccess::None => unreachable!(),
            }
            out += ",\"type\":";
            match &prop.ty {
                PropType::Bool(true) => out += r#"{"name":"bool","default":true}"#,
                PropType::Bool(false) => out += r#"{"name":"bool","default":false}"#,
                PropType::String(value) => {
                    out += r#"{"name":"string","default":"#;
                    append_json_string(&mut out, value);
                    out += "}";
                }
                PropType::Int(value, min, max) => {
                    out += r#"{"name":"integer","default":"#;
                    out += &value.to_string();
                    out += r#","min":"#;
                    out += &min.to_string();
                    out += r#","max":"#;
                    out += &max.to_string();
                    out += "}";
                }
                PropType::Uint(value, min, max) => {
                    out += r#"{"name":"integer","default":"#;
                    out += &value.to_string();
                    out += r#","min":"#;
                    out += &min.to_string();
                    out += r#","max":"#;
                    out += &max.to_string();
                    out += "}";
                }
                PropType::Datetime => out += r#"{"name":"string","default":""}"#,
                PropType::IntEnum(value, members) => {
                    out += r#"{"name":"enum","default":"#;
                    out += &value.to_string();
                    out += r#","members":["#;
                    for member in *members {
                        append_json_string(&mut out, member);
                        out += ",";
                    }
                    out.pop();
                    out += "]}";
                }
                PropType::StrEnum(value, members) => {
                    out += r#"{"name":"enum","default":"#;
                    append_json_string(&mut out, members[*value].1);
                    out += r#","members":["#;
                    for member in *members {
                        out += "[";
                        append_json_string(&mut out, member.0);
                        out += ",";
                        append_json_string(&mut out, member.1);
                        out += "],";
                    }
                    out.pop();
                    out += "]}";
                }
            }
            out += r#","label":"#;
            append_json_string(&mut out, prop.label);
            out += r#","desc":"#;
            append_json_string(&mut out, prop.desc);
            out += "},";
        }

        match out.pop() {
            Some('{') => out += "{}",
            Some(',') => out += "}",
            _ => unreachable!(),
        }
        out += "}";
        out
    }
    /// iterate over the names of all saves avaiable
    pub fn iter() -> Result<SaveIter, SaveError> {
        match std::fs::read_dir("saves") {
            Ok(paths) => Ok(SaveIter(Some(paths))),
            Err(_) => Err(SaveError::IOError),
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

/// reads the only the ones especified properties, if not found returns None
fn read_properties(path: impl AsRef<Path>) -> Result<HashMap<String, String>, SaveError> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let mut out = HashMap::new();

    let reader = BufReader::new(File::open(path)?);

    for line in reader.lines() {
        let line = line?;
        if !line.starts_with('#') {
            if let Some((key, value)) = line.split_once('=') {
                out.insert(key.trim().to_owned(), value.to_owned());
            }
        }
    }

    Ok(out)
}

/// writes the propertie to the file
fn write_properties(
    path: impl AsRef<Path> + Clone,
    mut values: HashMap<String, PropValue>,
) -> Result<(), SaveError> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // the contents of the entire file
    let mut out = String::with_capacity(4 * 1024);

    let reader = BufReader::new(File::open(path.clone())?);

    for line in reader.lines() {
        let line = line?;
        if !line.starts_with('#') {
            if let Some((raw_key, _)) = line.split_once('=') {
                if let Some(new_value) = values.remove(raw_key.trim()) {
                    out += raw_key;
                    out += "=";
                    new_value.to_prop_value(&mut out);
                    out += "\r\n";
                    continue;
                }
            }
        }
        out += &line;
        out += "\r\n";
    }

    for (key, value) in values {
        out += &key;
        out += "=";
        value.to_prop_value(&mut out);
        out += "\r\n";
    }

    std::fs::write(path, out)?;

    Ok(())
}

fn validate_properties(
    values: &HashMap<String, PropValue>,
    access_needed: PropAccess,
) -> Result<(), SaveError> {
    for (key, value) in values.iter() {
        if let Some(prop) = PROPERTIES.iter().filter(|prop| prop.name == key).next() {
            match (access_needed, prop.access) {
                (_, PropAccess::Write)
                | (PropAccess::None, _)
                | (PropAccess::Read, PropAccess::Read) => match &prop.ty {
                    PropType::Bool(_) => {
                        if let PropValue::Boolean(_) = value {
                            continue;
                        }
                    }
                    PropType::String(_) => {
                        if let PropValue::String(_) = value {
                            continue;
                        }
                    }
                    PropType::Int(_, min, max) => {
                        if let PropValue::Int(value) = value {
                            if value >= min && value <= max {
                                continue;
                            }
                        } else if let PropValue::Uint(value) = value {
                            if let Ok(value) = i64::try_from(*value) {
                                if value >= *min && value <= *max {
                                    continue;
                                }
                            }
                        }
                    }
                    PropType::Uint(_, min, max) => {
                        if let PropValue::Uint(value) = value {
                            if value >= min && value <= max {
                                continue;
                            }
                        } else if let PropValue::Int(value) = value {
                            if let Ok(value) = u64::try_from(*value) {
                                if value >= *min && value <= *max {
                                    continue;
                                }
                            }
                        }
                    }
                    PropType::Datetime => {
                        if let PropValue::String(value) = value {
                            if value.len() == 19 {
                                if let Ok(_) = chrono::NaiveDateTime::parse_from_str(
                                    value,
                                    "%Y-%m-%d %H:%M:%S",
                                ) {
                                    continue;
                                }
                            }
                        }
                    }
                    PropType::IntEnum(_, values) => {
                        if let PropValue::Int(value) = value {
                            if *value >= 0 && *value < values.len() as i64 {
                                continue;
                            }
                        } else if let PropValue::Uint(value) = value {
                            if *value < values.len() as u64 {
                                continue;
                            }
                        }
                    }
                    PropType::StrEnum(_, values) => {
                        if let PropValue::String(value) = value {
                            if values.iter().any(|(_, valid)| *valid == value) {
                                continue;
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        println!("Warning invalid property: {}", key);
        return Err(SaveError::PropertyNotFound);
    }
    Ok(())
}

fn folder_exists(path: impl AsRef<Path>) -> Result<(), SaveError> {
    match std::fs::metadata(path) {
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

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn append_json_string(out: &mut String, text: &str) {
    *out += "\"";
    for byte in text.bytes() {
        match byte {
            b'"' => *out += "\\\"",
            b'\\' => *out += "\\\\",
            7 => *out += "\\b",
            12 => *out += "\\f",
            b'\n' => *out += "\\n",
            b'\r' => *out += "\\r",
            b'\t' => *out += "\\t",
            0..=31 | 128..=255 => {
                *out += "\\x";
                let upper = byte >> 4;
                if upper < 10 {
                    out.push((b'0' + upper) as char);
                } else {
                    out.push((b'A' + upper - 10) as char);
                }
                let lower = byte & 0xF;
                if lower < 10 {
                    out.push((b'0' + lower) as char);
                } else {
                    out.push((b'A' + lower - 10) as char);
                }
            }
            _ => {
                out.push(byte as char);
            }
        }
    }
    *out += "\"";
}

impl PropValue {
    fn to_prop_value(&self, out: &mut String) {
        match self {
            PropValue::Boolean(true) => *out += "true",
            PropValue::Boolean(false) => *out += "false",
            PropValue::String(value) => *out += value,
            PropValue::Int(value) => *out += &value.to_string(),
            PropValue::Uint(value) => *out += &value.to_string(),
        }
    }
}

fn generate_properties(version: &str, values: &HashMap<String, PropValue>) -> String {
    let mut out = String::new();
    let now = now();
    for prop in PROPERTIES.iter() {
        if prop.access == PropAccess::None {
            continue;
        }
        out += prop.name;
        out += "=";
        if prop.name == "mc-manager-server-version" {
            out += version;
        } else if let (PropAccess::Write, Some(value)) = (prop.access, values.get(prop.name)) {
            value.to_prop_value(&mut out);
        } else {
            match &prop.ty {
                PropType::Bool(true) => out += "true",
                PropType::Bool(false) => out += "false",
                PropType::String(value) => out += value,
                PropType::Int(value, _, _) => out += &value.to_string(),
                PropType::Uint(value, _, _) => out += &value.to_string(),
                PropType::Datetime => out += &now,
                PropType::IntEnum(value, _) => out += &value.to_string(),
                PropType::StrEnum(value, members) => out += (*members)[*value].1,
            }
        }
        out += "\r\n";
    }
    out
}
