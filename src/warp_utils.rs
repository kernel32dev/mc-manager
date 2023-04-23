
macro_rules! get_filter {
    ($expr:expr) => {{
        use warp::Filter;
        warp::get()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($expr)
            )))
            .map($expr)
    }};
}

macro_rules! post_filter {
    ($ty:ty) => {{
        use warp::Filter;
        warp::post()
            .and(warp::path("api"))
            .and(warp::path(const_str::convert_ascii_case!(
                snake,
                stringify!($ty)
            )))
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .map(<$ty>::post)
    }};
}

macro_rules! or_filters {
    ($filter:expr) => {
        $filter
    };
    ($filter:expr, $($filters:expr,)*) => {
        $filter $(.or($filters))*
    };
}

macro_rules! catch {
    ($expr:expr) => {
        (|| $expr)()
    };
}

pub(crate) use get_filter;
pub(crate) use post_filter;
pub(crate) use or_filters;
pub(crate) use catch;

pub enum WarpResult<T: warp::Reply> {
    Ok(T),
    Err(warp::http::StatusCode),
}

impl<T> WarpResult<T>
where
    T: warp::Reply,
{
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
