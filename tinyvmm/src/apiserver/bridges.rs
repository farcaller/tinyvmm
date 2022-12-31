use actix_web::{delete, get, post, web, Responder};
use serde_valid::json::FromJsonValue;

use crate::database::{bridge::Bridge, entity::Entity};

#[get("")]
async fn list_bridges() -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = Bridge::list()?;

    Ok(web::Json(vms))
}

#[get("{name}")]
async fn get_bridge(path: web::Path<String>) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = Bridge::get(path.into_inner())?;

    Ok(web::Json(vms))
}

#[delete("{name}")]
async fn delete_bridge(
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    Bridge::delete(path.into_inner())?;

    Ok(web::Bytes::from(""))
}

#[post("")]
async fn create_bridge(
    vm: web::Json<serde_json::Value>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vm = Bridge::from_json_value(vm.0)?;
    vm.create()?;

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
