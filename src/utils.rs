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

macro_rules! catch {
    ($expr:expr) => {
        (|| $expr)()
    };
}

pub(crate) use catch;
pub(crate) use filters;

use warp::{reply::Response, Reply, hyper::StatusCode};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    BadRequest,
    NotFound,
    AlreadyExists,
    VersionNotFound,
    InvalidProperty,
    IsOnline,
    IsOffline,
    IsLoading,
    IsShutdown,
    PortInUse,
    IOError,
}

/// implements warp::Reply
pub enum WarpResult<T: Reply> {
    Ok(T),
    Err(SaveError),
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

impl Reply for SaveError
{
    fn into_response(self) -> Response {
        let body = match self {
            Self::BadRequest => return StatusCode::BAD_REQUEST.into_response(),
            Self::NotFound => r#"{"err":"NotFound","desc":"O save não foi encontrado"}"#,
            Self::AlreadyExists => r#"{"err":"AlreadyExists","desc":"O nome já é usado por um save"}"#,
            Self::VersionNotFound => r#"{"err":"VersionNotFound","desc":"A versão não existe, ou não está instalada"}"#,
            Self::InvalidProperty => r#"{"err":"InvalidProperty","desc":"Essa propiedade não existe ou o valor usado não é válido"}"#,
            Self::IsOnline => r#"{"err":"IsOnline","desc":"O save esta ligado"}"#,
            Self::IsOffline => r#"{"err":"IsOffline","desc":"O save esta desligado"}"#,
            Self::IsLoading => r#"{"err":"IsLoading","desc":"O save esta ligando"}"#,
            Self::IsShutdown => r#"{"err":"IsShutdown","desc":"O save esta desligando"}"#,
            Self::PortInUse => r#"{"err":"PortInUse","desc":"A porta já esta sendo usada por outro save"}"#,
            Self::IOError => r#"{"err":"IOError","desc":"Ocorreu um erro ao operar os arquivos"}"#,
        };
        let status = match self {
            Self::IOError => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        json_response_with_status(body.to_owned(), status)
    }
}

impl From<Result<(), SaveError>> for WarpResult<Response> {
    fn from(value: Result<(), SaveError>) -> Self {
        match value {
            Ok(()) => WarpResult::Ok(warp::reply().into_response()),
            Err(error) => WarpResult::Err(error),
        }
    }
}

impl<T: Reply> From<Result<T, SaveError>> for WarpResult<T> {
    fn from(value: Result<T, SaveError>) -> Self {
        match value {
            Ok(value) => WarpResult::Ok(value),
            Err(error) => WarpResult::Err(error),
        }
    }
}

/// creates a json reponse from raw body with the appropiate content-type
pub fn json_response(body: String) -> Response {
    use warp::http::header::CONTENT_TYPE;
    let (mut parts, body) = Response::new(body.into()).into_parts();
    parts
        .headers
        .append(CONTENT_TYPE, "application/json".parse().unwrap());
    Response::from_parts(parts, body)
}

/// creates a json reponse from raw body with the appropiate content-type
fn json_response_with_status(body: String, status: warp::http::StatusCode) -> Response {
    use warp::http::header::CONTENT_TYPE;
    let (mut parts, body) = Response::new(body.into()).into_parts();
    parts.status =status;
    parts
        .headers
        .append(CONTENT_TYPE, "application/json".parse().unwrap());
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
