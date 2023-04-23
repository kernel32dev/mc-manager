
mod api;
mod warp_utils;

use warp::Filter;
use api::*;
use warp_utils::{get_filter, post_filter, or_filters};

const PORT: u16 = 3030;

#[tokio::main]
async fn main() {
    let apis = or_filters!(
        get_filter!(versions),
        get_filter!(saves),
        post_filter!(CreateWorld),
    );

    #[cfg(not(debug_assertions))]
    let routes = apis.or(static_dir::static_dir!("static"));

    #[cfg(debug_assertions)]
    let routes = apis.or(warp::fs::dir("static"));

    warp::serve(routes).run(([127, 0, 0, 1], PORT)).await;
}
