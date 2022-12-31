use actix_web::{App, HttpServer};
use log::info;

mod bridges;
mod virtualmachines;

pub async fn run_server<P>(uds_path: P) -> std::io::Result<()>
where
    P: AsRef<std::path::Path>,
{
    info!("starting the api server");
    HttpServer::new(|| {
        App::new()
            .configure(virtualmachines::vms_apis)
            .configure(bridges::bridges_apis)
    })
    .bind_uds(uds_path)?
    .run()
    .await
}
