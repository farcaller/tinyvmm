use rand::prelude::*;

pub type VirtualMachine = res::v1alpha2::VirtualMachine;

mod res {
    pub mod v1alpha1 {
        use serde_valid::json::FromJsonValue;
        use vmm_entity::{vmm_entity, vmm_entity_struct};

        use crate::database::error::Error;

        #[vmm_entity("v1alpha1")]
        pub struct VirtualMachine {
            #[validate]
            pub spec: VirtualMachineSpec,
        }

        #[vmm_entity_struct]
        pub struct VirtualMachineSpec {
            #[validate(minimum = 1)]
            pub cpus: u8,
            #[validate(pattern = r"^\d+(M|G)$")]
            pub memory: String,
            #[validate(custom(super::super::disk_path_validation))]
            pub disk: String,
            #[validate(pattern = r"^\d+\.\d+\.\d+\.\d+$")]
            pub ip: String,
            #[validate(
                pattern = r"^[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}$"
            )]
            #[serde(default = "super::super::generate_default_mac")]
            pub mac: String,
        }

        impl VirtualMachine {
            #[allow(dead_code)] // TODO: migration
            pub fn migrate(self) -> Result<super::v1alpha2::VirtualMachine, Error> {
                let mut spec = serde_json::to_value(self.spec)?;
                let m = if let serde_json::Value::Object(ref mut m) = spec {
                    m
                } else {
                    return Err(Error::NotAnObject);
                };
                m.insert("bridge".into(), "tvbr0".into());

                Ok(super::v1alpha2::VirtualMachine {
                    metadata: self.metadata,
                    spec: super::v1alpha2::VirtualMachineSpec::from_json_value(spec)?,

                    ..Default::default()
                })
            }
        }
    }

    pub mod v1alpha2 {
        use vmm_entity::{vmm_entity, vmm_entity_struct};

        #[vmm_entity("v1alpha2")]
        pub struct VirtualMachine {
            #[validate]
            pub spec: VirtualMachineSpec,
        }

        #[vmm_entity_struct]
        pub struct VirtualMachineSpec {
            #[validate(minimum = 1)]
            pub cpus: u8,
            #[validate(pattern = r"^\d+(M|G)$")]
            pub memory: String,
            #[validate(custom(super::super::disk_path_validation))]
            pub disk: String,
            #[validate(pattern = r"^\d+\.\d+\.\d+\.\d+$")]
            pub ip: String,
            #[validate(
                pattern = r"^[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}$"
            )]
            #[serde(default = "super::super::generate_default_mac")]
            pub mac: String,
            #[validate(
                pattern = r"^[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*$"
            )]
            pub bridge: String,
        }
    }
}

fn disk_path_validation(path_str: &str) -> Result<(), serde_valid::validation::Error> {
    let path = std::path::PathBuf::from(path_str);
    match path.exists() {
        true => Ok(()),
        false => Err(serde_valid::validation::Error::Custom(format!(
            "disk path `{}` doesn't exist",
            path_str
        ))),
    }
}
fn generate_default_mac() -> String {
    let mut data = [0u8; 6];
    rand::thread_rng().fill_bytes(&mut data);

    format!(
        "66:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        data[1] as usize, data[2] as usize, data[3] as usize, data[4] as usize, data[5] as usize
    )
}
