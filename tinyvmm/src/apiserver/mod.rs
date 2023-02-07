use actix_web::{web, App, HttpServer};
use log::info;

mod bridges;
mod virtualmachines;

pub(crate) struct RuntimeDir(String);
pub async fn run_server<P>(uds_path: P, runtime_dir: String) -> std::io::Result<()>
where
    P: AsRef<std::path::Path>,
{
    info!("starting the api server");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(RuntimeDir(runtime_dir.clone())))
            .configure(virtualmachines::vms_apis)
            .configure(bridges::bridges_apis)
    })
    .bind_uds(uds_path)?
    .run()
    .await
}
