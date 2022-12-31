use vmm_entity::{vmm_entity, vmm_entity_struct};

#[vmm_entity("v1alpha1")]
pub struct Bridge {
    #[validate]
    pub spec: BridgeSpec,
}

#[vmm_entity_struct]
pub struct BridgeSpec {
    #[validate(pattern = r"^\d+\.\d+\.\d+\.\d+/\d+$")]
    pub address: String,
    #[validate(pattern = r"^[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*$")]
    pub dns_zone: String,
    #[validate(pattern = r"^\d+\.\d+\.\d+\.\d+$")]
    pub dns_server: String,
}
