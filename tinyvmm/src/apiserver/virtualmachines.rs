use actix_web::{delete, get, post, web, Responder};
use serde_valid::json::FromJsonValue;

use crate::database::{entity::Entity, store::Store, virtual_machine::VirtualMachine};

#[get("")]
async fn list_vms(store: web::Data<Store>) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = VirtualMachine::list(&store)?;

    Ok(web::Json(vms))
}

#[get("{name}")]
async fn get_vm(
    store: web::Data<Store>,
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vms = VirtualMachine::get(&store, path.into_inner())?;

    Ok(web::Json(vms))
}

#[delete("{name}")]
async fn delete_vm(
    store: web::Data<Store>,
    path: web::Path<String>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    VirtualMachine::delete(&store, path.into_inner())?;

    Ok(web::Bytes::from(""))
}

#[post("")]
async fn create_vm(
    store: web::Data<Store>,
    vm: web::Json<serde_json::Value>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
    let vm = VirtualMachine::from_json_value(vm.0)?;
    vm.create(&store)?;

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
