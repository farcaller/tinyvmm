use actix_web::{delete, get, post, web, Responder};
use serde_valid::json::FromJsonValue;

use crate::{
    apiserver::RuntimeDir,
    database::{entity::Entity, virtual_machine::VirtualMachine},
};

#[get("")]
async fn list_vms(
    runtime_dir: web::Data<RuntimeDir>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = VirtualMachine::list(&runtime_dir.0)?;

    Ok(web::Json(vms))
}

#[get("{name}")]
async fn get_vm(
    runtime_dir: web::Data<RuntimeDir>,
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = VirtualMachine::get(&runtime_dir.0, path.into_inner())?;

    Ok(web::Json(vms))
}

#[delete("{name}")]
async fn delete_vm(
    runtime_dir: web::Data<RuntimeDir>,
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    VirtualMachine::delete(&runtime_dir.0, path.into_inner())?;

    Ok(web::Bytes::from(""))
}

#[post("")]
async fn create_vm(
    runtime_dir: web::Data<RuntimeDir>,
    vm: web::Json<serde_json::Value>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vm = VirtualMachine::from_json_value(vm.0)?;
    vm.create(&runtime_dir.0)?;

    Ok(web::Json(vm))
}

pub fn vms_apis(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/virtualmachines")
            .service(list_vms)
            .service(create_vm)
            .service(get_vm)
            .service(delete_vm),
    );
}
