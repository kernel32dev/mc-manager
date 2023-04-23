
mod api;
mod warp_utils;

use warp::Filter;
use api::*;
use warp_utils::filters;

const PORT: u16 = 3030;

#[tokio::main]
async fn main() {
    let apis = filters!(
        GET versions;
        GET saves;
        POST CreateWorld;
    );

    #[cfg(not(debug_assertions))]
    let routes = apis.or(static_dir::static_dir!("static"));

    #[cfg(debug_assertions)]
    let routes = apis.or(warp::fs::dir("static"));

    warp::serve(routes).run(([127, 0, 0, 1], PORT)).await;
}
