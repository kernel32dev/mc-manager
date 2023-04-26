
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
use warp::Reply;
use crate::state::SaveError;

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

impl WarpResult<warp::reply::Response> {
    pub fn reply() -> Self {
        WarpResult::Reply(warp::reply().into_response())
    }
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
        match self {
            WarpResult::Reply(reply) => reply.into_response(),
            WarpResult::Status(status) => status.into_response(),
            WarpResult::SaveError(error) => match error {
                SaveError::NotFound => warp::reply::with_status(warp::reply::json(&NOTFOUND), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::AlreadyExists => warp::reply::with_status(warp::reply::json(&ALREADYEXISTS), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::VersionNotFound => warp::reply::with_status(warp::reply::json(&VERSIONNOTFOUND), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::PropertyNotFound => warp::reply::with_status(warp::reply::json(&PROPERTYNOTFOUND), warp::http::StatusCode::BAD_REQUEST).into_response(),
                SaveError::IOError => warp::reply::with_status(warp::reply::json(&IOERROR), warp::http::StatusCode::INTERNAL_SERVER_ERROR).into_response(),
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
