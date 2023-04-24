
use warp::Filter;
use crate::api::*;
use crate::warp_utils::filters;

const PORT: u16 = 3030;

pub fn serve(shutdown: Option<tokio::sync::oneshot::Receiver<()>>) {
    let apis = filters!(
        GET versions;
        GET saves;
        GET icons String;
        POST CreateWorld;
    );

    #[cfg(not(debug_assertions))]
    let routes = apis.or(static_dir::static_dir!("static"));

    #[cfg(debug_assertions)]
    let routes = apis.or(warp::fs::dir("static"));

    let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();
    let _enter = rt.enter();
    rt.block_on(
        warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], PORT), async move {
            if let Some(shutdown) = shutdown {
                shutdown.await.unwrap();
            } else {
                tokio::signal::ctrl_c().await.unwrap();
            }
        }).1
    );
}
