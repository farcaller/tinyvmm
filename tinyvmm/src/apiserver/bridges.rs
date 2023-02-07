use actix_web::{delete, get, post, web, Responder};
use serde_valid::json::FromJsonValue;

use crate::{
    apiserver::RuntimeDir,
    database::{bridge::Bridge, entity::Entity},
};

#[get("")]
async fn list_bridges(
    runtime_dir: web::Data<RuntimeDir>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = Bridge::list(&runtime_dir.0)?;

    Ok(web::Json(vms))
}

#[get("{name}")]
async fn get_bridge(
    runtime_dir: web::Data<RuntimeDir>,
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = Bridge::get(&runtime_dir.0, path.into_inner())?;

    Ok(web::Json(vms))
}

#[delete("{name}")]
async fn delete_bridge(
    runtime_dir: web::Data<RuntimeDir>,
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    Bridge::delete(&runtime_dir.0, path.into_inner())?;

    Ok(web::Bytes::from(""))
}

#[post("")]
async fn create_bridge(
    runtime_dir: web::Data<RuntimeDir>,
    vm: web::Json<serde_json::Value>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vm = Bridge::from_json_value(vm.0)?;
    vm.create(&runtime_dir.0)?;

    Ok(web::Json(vm))
}

pub fn bridges_apis(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/bridges")
            .service(list_bridges)
            .service(create_bridge)
            .service(get_bridge)
            .service(delete_bridge),
    );
}
