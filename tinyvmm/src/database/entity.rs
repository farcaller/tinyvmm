use serde::{de::DeserializeOwned, Serialize};

use super::{create_entity, delete_entity, get_entity, get_kind};

pub trait Entity {
    const KIND: &'static str;
    const API_VERSION: &'static str;
    type Type: DeserializeOwned;

    fn get<T>(runtime_dir: &str, name: T) -> Result<Self::Type, super::error::Error>
    where
        T: AsRef<str>,
    {
        let entity = get_entity(runtime_dir, Self::KIND, name.as_ref())?;
        let unwrapped: Self::Type = serde_json::value::from_value(entity)?;

        Ok(unwrapped)
    }

    fn delete<T>(runtime_dir: &str, name: T) -> Result<(), super::error::Error>
    where
        T: AsRef<str>,
    {
        delete_entity(runtime_dir, Self::KIND, name.as_ref())?;

        Ok(())
    }

    fn list(runtime_dir: &str) -> Result<Vec<Self::Type>, super::error::Error> {
        let entities = get_kind(runtime_dir, Self::KIND)?;

        let mut vms = vec![];
        for e in entities {
            let vm = serde_json::value::from_value(e)?;
            vms.push(vm);
        }

        Ok(vms)
    }

    fn create(&self, runtime_dir: &str) -> Result<(), super::error::Error>
    where
        Self: Serialize,
    {
        let val = serde_json::value::to_value(self)?;
        create_entity(runtime_dir, val)?;
        Ok(())
    }
}
