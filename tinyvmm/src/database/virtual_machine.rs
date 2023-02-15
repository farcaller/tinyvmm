use rand::prelude::*;

pub type VirtualMachine = res::v1alpha3::VirtualMachine;

mod res {
    use serde_json::value::Value;

    use crate::database::{entity::MigratableEntity, error::Error};

    pub fn get_migrator(version: &str) -> Option<fn(Value) -> Result<Value, Error>> {
        match version {
            "v1alpha1" => Some(v1alpha1::VirtualMachine::migrate),
            "v1alpha2" => Some(v1alpha2::VirtualMachine::migrate),
            "v1alpha3" => Some(v1alpha3::VirtualMachine::migrate),
            _ => None,
        }
    }

    pub mod v1alpha1 {
        use serde_json::{json, value::Value};
        use vmm_entity::{vmm_entity, vmm_entity_struct};

        use super::get_migrator;

        use crate::database::{
            entity::{Entity, MigratableEntity},
            error::Error,
            serde::{EntityObject, ValueGetter},
        };

        #[vmm_entity("v1alpha1", "get_migrator")]
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

        impl MigratableEntity for VirtualMachine {
            fn migrate(entity: Value) -> Result<Value, Error> {
                let spec = entity.get_existing("spec")?.as_map()?;
                let mut new_spec = spec.clone();
                new_spec.insert("bridge".into(), "tvbr0".into());

                Ok(json! ({
                    "apiVersion": super::v1alpha2::VirtualMachine::API_VERSION,
                    "kind": Self::KIND,
                    "metadata": entity.get("metadata").ok_or(Error::MissingKey("metadata"))?,
                    "spec": new_spec,
                }))
            }
        }
    }

    pub mod v1alpha2 {
        use serde_json::{json, value::Value};
        use vmm_entity::{vmm_entity, vmm_entity_struct};

        use super::get_migrator;

        use crate::database::{
            entity::{Entity, MigratableEntity},
            error::Error,
            serde::{EntityObject, ValueGetter},
        };

        #[vmm_entity("v1alpha2", "get_migrator")]
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

        impl MigratableEntity for VirtualMachine {
            fn migrate(entity: Value) -> Result<Value, Error> {
                let spec = entity.get_existing("spec")?.as_map()?;
                let mut new_spec = spec.clone();
                new_spec.insert(
                    "disks".into(),
                    vec![spec.get_existing("disk")?.clone()].into(),
                );
                new_spec.remove("disk");

                Ok(json! ({
                    "apiVersion": super::v1alpha3::VirtualMachine::API_VERSION,
                    "kind": Self::KIND,
                    "metadata": entity.get_existing("metadata")?,
                    "spec": new_spec,
                }))
            }
        }
    }

    pub mod v1alpha3 {
        use serde_json::value::Value;
        use vmm_entity::{vmm_entity, vmm_entity_struct};

        use super::get_migrator;

        use crate::database::{
            entity::{Entity, MigratableEntity},
            error::Error,
        };

        #[vmm_entity("v1alpha3", "get_migrator")]
        pub struct VirtualMachine {
            #[validate]
            pub spec: VirtualMachineSpec,
        }

        impl MigratableEntity for VirtualMachine {
            fn migrate(_entity: Value) -> Result<Value, Error> {
                Err(Error::NoMigrationAvailable {
                    kind: Self::KIND,
                    version: Self::API_VERSION,
                })
            }
        }

        #[vmm_entity_struct]
        pub struct VirtualMachineSpec {
            #[validate(minimum = 1)]
            pub cpus: u8,
            #[validate(pattern = r"^\d+(M|G)$")]
            pub memory: String,
            #[validate(custom(super::super::disks_path_validation))]
            pub disks: Vec<String>,
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

fn disks_path_validation(paths: &Vec<String>) -> Result<(), serde_valid::validation::Error> {
    for path_str in paths {
        let path = std::path::PathBuf::from(path_str);
        if !path.exists() {
            return Err(serde_valid::validation::Error::Custom(format!(
                "disk path `{}` doesn't exist",
                path_str
            )));
        }
    }
    Ok(())
}

fn disk_path_validation(path_str: &str) -> Result<(), serde_valid::validation::Error> {
    disks_path_validation(&vec![path_str.into()])
}

fn generate_default_mac() -> String {
    let mut data = [0u8; 6];
    rand::thread_rng().fill_bytes(&mut data);

    format!(
        "66:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        data[1] as usize, data[2] as usize, data[3] as usize, data[4] as usize, data[5] as usize
    )
}
