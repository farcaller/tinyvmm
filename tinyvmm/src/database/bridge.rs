use serde_json::value::Value;
use vmm_entity::{vmm_entity, vmm_entity_struct};

use super::{entity::MigratableEntity, error::Error};

pub fn get_migrator(version: &str) -> Option<fn(Value) -> Result<Value, Error>> {
    match version {
        "v1alpha1" => Some(Bridge::migrate),
        _ => None,
    }
}

#[vmm_entity("v1alpha1", "get_migrator")]
pub struct Bridge {
    #[validate]
    pub spec: BridgeSpec,
}
impl MigratableEntity for Bridge {}

#[vmm_entity_struct]
pub struct BridgeSpec {
    #[validate(pattern = r"^\d+\.\d+\.\d+\.\d+/\d+$")]
    pub address: String,
    #[validate(pattern = r"^[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*$")]
    pub dns_zone: String,
    #[validate(pattern = r"^\d+\.\d+\.\d+\.\d+$")]
    pub dns_server: String,
}
