
macro_rules! filters {
    (GET $func_name:ident $($ty:ty)*;) => {{
        use warp::Filter;
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
    (GET $func_name:ident $($ty:ty)*; $($tail:tt)+) => {
        filters!(GET $func_name $($ty)*;).or(filters!($($tail)+))
    };
    (POST $struct_name:ident $($ty:ty)*;) => {{
        use warp::Filter;
        warp::post()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($struct_name)
            )))
            $(.and(warp::path::param::<$ty>()))*
            .and(warp::path::end())
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .map(<$struct_name>::post)
    }};
    (POST $struct_name:ident $($ty:ty)*; $($tail:tt)+) => {
        filters!(POST $struct_name $($ty)*;).or(filters!($($tail)+))
    };
}

macro_rules! catch {
    ($expr:expr) => {
        (|| $expr)()
    };
}

pub(crate) use filters;
pub(crate) use catch;

use crate::state::SaveError;
use warp::reply::Response;

/// implements warp::Reply if T also implements warp::Reply
pub enum WarpResult<T> {
    Reply(T),
    Status(warp::http::StatusCode),
    SaveError(SaveError),
}

impl<T> WarpResult<T>
{
    pub const BAD_REQUEST: Self =
        WarpResult::Status(warp::http::StatusCode::BAD_REQUEST);
    pub const INTERNAL_SERVER_ERROR: Self =
        WarpResult::Status(warp::http::StatusCode::INTERNAL_SERVER_ERROR);
}

impl<T> warp::Reply for WarpResult<T>
where
    T: warp::Reply,
{
    fn into_response(self) -> warp::reply::Response {
        #[derive(serde::Serialize)]
        struct SaveErrorJson {
            err: &'static str,
            desc: &'static str,
        }
        const NOTFOUND: SaveErrorJson = SaveErrorJson { err: "NotFound", desc: "O save não foi encontrado" };
        const ALREADYEXISTS: SaveErrorJson = SaveErrorJson { err: "AlreadyExists", desc: "O nome já é usado por um save" };
        const VERSIONNOTFOUND: SaveErrorJson = SaveErrorJson { err: "VersionNotFound", desc: "A versão não existe, ou não está instalada" };
        const PROPERTYNOTFOUND: SaveErrorJson = SaveErrorJson { err: "PropertyNotFound", desc: "Essa propiedade não existe" };
        const IOERROR: SaveErrorJson = SaveErrorJson { err: "IOError", desc: "Ocorreu um erro ao operar os arquivos" };
        const ISONLINE: SaveErrorJson = SaveErrorJson { err: "IsOnline", desc: "O save esta ligado" };
        const ISOFFLINE: SaveErrorJson = SaveErrorJson { err: "IsOffline", desc: "O save esta desligado" };
        const ISLOADING: SaveErrorJson = SaveErrorJson { err: "IsLoading", desc: "O save esta ligando" };
        match self {
            WarpResult::Reply(reply) => reply.into_response(),
            WarpResult::Status(status) => status.into_response(),
            WarpResult::SaveError(error) => match error {
                SaveError::NotFound => warp::reply::with_status(warp::reply::json(&NOTFOUND), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::AlreadyExists => warp::reply::with_status(warp::reply::json(&ALREADYEXISTS), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::VersionNotFound => warp::reply::with_status(warp::reply::json(&VERSIONNOTFOUND), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::PropertyNotFound => warp::reply::with_status(warp::reply::json(&PROPERTYNOTFOUND), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::IOError => warp::reply::with_status(warp::reply::json(&IOERROR), warp::http::StatusCode::INTERNAL_SERVER_ERROR).into_response(),
                SaveError::IsOnline => warp::reply::with_status(warp::reply::json(&ISONLINE), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::IsOffline => warp::reply::with_status(warp::reply::json(&ISOFFLINE), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::IsLoading => warp::reply::with_status(warp::reply::json(&ISLOADING), warp::http::StatusCode::BAD_REQUEST).into_response(),
            },
        }
    }
}

impl<T> From<Result<T, warp::http::StatusCode>> for WarpResult<T> {
    fn from(value: Result<T, warp::http::StatusCode>) -> Self {
        match value {
            Ok(value) => WarpResult::Reply(value),
            Err(status) => WarpResult::Status(status),
        }
    }
}

impl<T> From<Result<T, SaveError>> for WarpResult<T> {
    fn from(value: Result<T, SaveError>) -> Self {
        match value {
            Ok(value) => WarpResult::Reply(value),
            Err(error) => WarpResult::SaveError(error),
        }
    }
}

/// creates a json reponse from raw body with the appropiate content-type
pub fn json_response(body: String) -> Response {
    use warp::http::header::CONTENT_TYPE;
    let (mut parts, body) = Response::new(body.into()).into_parts();
    parts.headers.append(CONTENT_TYPE, "application/json".parse().unwrap());
    Response::from_parts(parts, body)
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

pub fn append_comma_separated<T>(mut iter: impl Iterator<Item = T>, out: &mut String, mut callback: impl FnMut(&mut String, T)) {
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
