macro_rules! filters {
    (GET async fn $func_name:ident $($ty:ty)*;) => {{
        warp::get()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($func_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .and_then($func_name)
    }};
    (GET fn $func_name:ident $($ty:ty)*;) => {{
        warp::get()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($func_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .map($func_name)
    }};
    (POST async fn $func_name:ident $($ty:ty)*;) => {{
        warp::post()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($func_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .and_then($func_name)
    }};
    (POST fn $func_name:ident $($ty:ty)*;) => {{
        warp::post()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($func_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .map($func_name)
    }};
    (WS async fn $func_name:ident $($ty:ty)*;) => {{
        warp::path("api")
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($func_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .and(warp::ws())
            .and_then($func_name)
    }};
    (WS fn $func_name:ident $($ty:ty)*;) => {{
        warp::path("api")
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($func_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .and(warp::ws())
            .map($func_name)
    }};
    ($verb:ident async fn $func_name:ident $($ty:ty)*; $($tail:tt)+) => {
        filters!($verb async fn $func_name $($ty)*;).or(filters!($($tail)+))
    };
    ($verb:ident fn $func_name:ident $($ty:ty)*; $($tail:tt)+) => {
        filters!($verb fn $func_name $($ty)*;).or(filters!($($tail)+))
    };
}

pub(crate) use filters;

use warp::{reply::Response, Reply, hyper::StatusCode};

use crate::instances::{InstanceStatus, get_java_path};

#[derive(Clone, PartialEq, Eq)]
pub enum ApiError {
    BadRequest,
    BadName,
    NotFound,
    AlreadyExists,
    VersionNotFound,
    PropertyNotFound(String),
    PropertyReadOnly(String),
    PropertyInvalid(String),
    BadConfig(String),
    BadInstanceStatus(InstanceStatus),
    PortInUse,
    JavaError(String),
    IOError(String),
}

/// implements warp::Reply
pub enum WarpResult<T: Reply> {
    Ok(T),
    Err(ApiError),
}

impl<T: Reply> Reply for WarpResult<T>
{
    fn into_response(self) -> Response {
        match self {
            WarpResult::Ok(reply) => reply.into_response(),
            WarpResult::Err(error) => error.into_response(),
        }
    }
}

impl Reply for ApiError
{
    fn into_response(self) -> Response {
        let status = match self {
            Self::IOError(_) | Self::JavaError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        let body = match self {
            Self::BadRequest => return StatusCode::BAD_REQUEST.into_response(),
            Self::BadName => r#"{"err":"BadName","desc":"Esse nome não pode ser usado como nome de um mundo"}"#.to_owned(),
            Self::NotFound => r#"{"err":"NotFound","desc":"O save não foi encontrado"}"#.to_owned(),
            Self::AlreadyExists => r#"{"err":"AlreadyExists","desc":"O nome já é usado por um save"}"#.to_owned(),
            Self::VersionNotFound => r#"{"err":"VersionNotFound","desc":"A versão não existe, ou não está instalada"}"#.to_owned(),
            Self::PropertyNotFound(prop) => {
                let mut out = String::with_capacity(256);
                out.push_str(r#"{"err":"PropertyNotFound","desc":"Essa propiedade não existe"},"prop":"#);
                append_json_string(&mut out, &prop);
                out.push('}');
                out
            },
            Self::PropertyReadOnly(prop) => {
                let mut out = String::with_capacity(256);
                out.push_str(r#"{"err":"PropertyReadOnly","desc":"Não é possível escrever para esta propiedade"},"prop":"#);
                append_json_string(&mut out, &prop);
                out.push('}');
                out
            },
            Self::PropertyInvalid(prop) => {
                let mut out = String::with_capacity(256);
                out.push_str(r#"{"err":"PropertyInvalid","desc":"O valor usada para essa propieadade é inválido"},"prop":"#);
                append_json_string(&mut out, &prop);
                out.push('}');
                out
            },
            Self::BadConfig(prop) => {
                let mut out = String::with_capacity(256);
                out.push_str(r#"{"err":"BadConfig","desc":"Essa propieadade está configurada com um valor inválido, reconfigure com um valor válido"},"prop":"#);
                append_json_string(&mut out, &prop);
                out.push('}');
                out
            },
            Self::BadInstanceStatus(status) => match status {
                InstanceStatus::Cold => r#"{"err":"BadInstanceStatus","desc":"O save está desligado","status":"cold"}"#,
                InstanceStatus::Loading => r#"{"err":"BadInstanceStatus","desc":"O save está ligando","status":"loading"}"#,
                InstanceStatus::Online => r#"{"err":"BadInstanceStatus","desc":"O save está ligado","status":"online"}"#,
                InstanceStatus::Shutdown => r#"{"err":"BadInstanceStatus","desc":"O save está desligando","status":"shutdown"}"#,
                InstanceStatus::Offline => r#"{"err":"BadInstanceStatus","desc":"O save está desligado","status":"offline"}"#,
            }.to_owned(),
            Self::PortInUse => r#"{"err":"PortInUse","desc":"A porta já esta sendo usada por outro save"}"#.to_owned(),
            Self::JavaError(desc) => {
                let mut out = String::with_capacity(256);
                out.push_str(r#"{"err":"JavaError","desc":"Ocorreu um erro ao executar o Java","ioerr":"#);
                append_json_string(&mut out, &desc);
                out.push('}');
                out
            },
            Self::IOError(desc) => {
                let mut out = String::with_capacity(256);
                out.push_str(r#"{"err":"IOError","desc":"Ocorreu um erro ao operar os arquivos","ioerr":"#);
                append_json_string(&mut out, &desc);
                out.push_str(r#""java":"#);
                append_json_string(&mut out, get_java_path().as_str());
                out.push('}');
                out
            },
        };
        json_response_with_status(body, status)
    }
}

impl From<Result<(), ApiError>> for WarpResult<Response> {
    fn from(value: Result<(), ApiError>) -> Self {
        match value {
            Ok(()) => WarpResult::Ok(warp::reply().into_response()),
            Err(error) => WarpResult::Err(error),
        }
    }
}

impl<T: Reply> From<Result<T, ApiError>> for WarpResult<T> {
    fn from(value: Result<T, ApiError>) -> Self {
        match value {
            Ok(value) => WarpResult::Ok(value),
            Err(error) => WarpResult::Err(error),
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        Self::IOError(error.to_string())
    }
}

/// creates a json reponse from raw body with the appropiate content-type
pub fn json_response(body: String) -> Response {
    use warp::http::header::CONTENT_TYPE;
    let (mut parts, body) = Response::new(body.into()).into_parts();
    parts
        .headers
        .append(CONTENT_TYPE, "application/json".parse().expect("\"application/json\" is not a valid content-type"));
    Response::from_parts(parts, body)
}

/// creates a json reponse from raw body with the appropiate content-type
fn json_response_with_status(body: String, status: warp::http::StatusCode) -> Response {
    use warp::http::header::CONTENT_TYPE;
    let (mut parts, body) = Response::new(body.into()).into_parts();
    parts.status =status;
    parts
        .headers
        .append(CONTENT_TYPE, "application/json".parse().expect("\"application/json\" is not a valid content-type"));
    Response::from_parts(parts, body)
}

pub fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn append_json_string(out: &mut String, text: &str) {
    *out += "\"";
    for char in text.chars() {
        match char {
            '"' => *out += r#"\""#,
            '\\' => *out += r"\\",
            '\x07' => *out += r"\b",
            '\x0C' => *out += r"\f",
            '\n' => *out += r"\n",
            '\r' => *out += r"\r",
            '\t' => *out += r"\t",
            '\0'..='\x1F' | '\x7F' => {
                *out += r"\x";
                let upper = char as u8 >> 4;
                if upper < 10 {
                    out.push((b'0' + upper) as char);
                } else {
                    out.push((b'A' + upper - 10) as char);
                }
                let lower = char as u8 & 0xF;
                if lower < 10 {
                    out.push((b'0' + lower) as char);
                } else {
                    out.push((b'A' + lower - 10) as char);
                }
            }
            _ => out.push(char),
        }
    }
    *out += "\"";
}

pub fn append_comma_separated<T>(
    mut iter: impl Iterator<Item = T>,
    out: &mut String,
    mut callback: impl FnMut(&mut String, T),
) {
    let mut at_least_one_comma = false;
    while let Some(next) = iter.next() {
        let last_size = out.len();
        callback(out, next);
        if out.len() != last_size {
            out.push(',');
            at_least_one_comma = true;
        }
    }
    if at_least_one_comma {
        out.pop();
    }
}
