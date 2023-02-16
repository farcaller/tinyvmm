use actix_web::{web, App, HttpServer};
use eyre::Context;
use log::info;

use crate::database::store::Store;

mod bridges;
mod virtualmachines;

pub async fn run_server<P>(uds_path: P, store: Store) -> eyre::Result<()>
where
    P: AsRef<std::path::Path>,
{
    info!("starting the api server");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(store.clone()))
            .configure(virtualmachines::vms_apis)
            .configure(bridges::bridges_apis)
    })
    .bind_uds(uds_path)
    .wrap_err("failed to bind the api server listener")?;

    server.run().await.wrap_err("failed to run the api server")
}
