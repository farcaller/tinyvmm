use serde::{de::DeserializeOwned, Serialize};
use serde_json::value::Value;

use super::{create_entity, delete_entity, error::Error, get_entity, get_kind, serde::ValueGetter};

pub trait Entity {
    const KIND: &'static str;
    const API_VERSION: &'static str;
    type Type: DeserializeOwned + MigratableEntity;

    fn migrator(version: &str) -> Option<fn(Value) -> Result<Value, Error>>;

    fn migrate_version(mut entity: Value) -> Result<Value, Error> {
        loop {
            let version = entity
                .get_existing("apiVersion")?
                .as_str()
                .ok_or(Error::MissingKey("apiVersion"))?;
            if version == Self::API_VERSION {
                return Ok(entity);
            }
            let migrator = Self::migrator(version).ok_or(Error::FailedMigration {
                kind: Self::KIND,
                from: version.to_string(),
                to: None,
            })?;
            entity = migrator(entity)?;
        }
    }

    fn get<T>(runtime_dir: &str, name: T) -> Result<Self::Type, super::error::Error>
    where
        T: AsRef<str>,
    {
        let entity = get_entity(runtime_dir, Self::KIND, name.as_ref())?;
        let entity = Self::migrate_version(entity)?;

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
        for entity in entities {
            let entity = Self::migrate_version(entity)?;
            let vm = serde_json::value::from_value(entity)?;
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

pub trait MigratableEntity {
    fn migrate(entity: Value) -> Result<Value, super::error::Error> {
        Ok(entity)
    }
}
