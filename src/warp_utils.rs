
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

pub enum WarpResult<T: warp::Reply> {
    Ok(T),
    Err(warp::http::StatusCode),
}

impl<T> WarpResult<T>
where
    T: warp::Reply,
{
    pub const BAD_REQUEST: Self =
        WarpResult::Err(warp::http::StatusCode::BAD_REQUEST);
    pub const INTERNAL_SERVER_ERROR: Self =
        WarpResult::Err(warp::http::StatusCode::INTERNAL_SERVER_ERROR);
}

impl<T> warp::Reply for WarpResult<T>
where
    T: warp::Reply,
{
    fn into_response(self) -> warp::reply::Response {
        match self {
            WarpResult::Ok(reply) => reply.into_response(),
            WarpResult::Err(status) => status.into_response(),
        }
    }
}
