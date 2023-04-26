
use warp::Filter;
use crate::api::*;
use crate::warp_utils::filters;

const PORT: u16 = 3030;

pub fn serve(shutdown: Option<tokio::sync::oneshot::Receiver<()>>) {
    let apis = filters!(
        GET versions;
        GET saves;
        GET icons String;
        POST CreateSave;
        POST ModifySave;
        POST DeleteSave;
    );

    #[cfg(not(debug_assertions))] // cd into folder of executable
    std::env::set_current_dir(std::env::current_exe().expect("current_exe").parent().unwrap()).expect("set_current_dir");

    #[cfg(not(debug_assertions))] // load assets from executable
    let routes = apis.or(static_dir::static_dir!("static"));

    #[cfg(debug_assertions)] // cd into folder of project (leave target/debug/)
    std::env::set_current_dir(std::env::current_exe().expect("current_exe").parent().unwrap().parent().unwrap().parent().unwrap()).expect("set_current_dir");

    #[cfg(debug_assertions)] // load assets from static directory
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