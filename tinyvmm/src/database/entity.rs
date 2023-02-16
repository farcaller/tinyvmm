use serde::{de::DeserializeOwned, Serialize};
use serde_json::value::Value;

use super::{error::Error, serde::ValueGetter, store::Store};

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

    fn get<T>(store: &Store, name: T) -> Result<Self::Type, super::error::Error>
    where
        T: AsRef<str>,
    {
        let entity = store
            .get_entity(Self::KIND, name.as_ref())
            .and_then(|e| e.ok_or(Error::NotFound))?;
        let entity = Self::migrate_version(entity)?;

        let unwrapped: Self::Type = serde_json::value::from_value(entity)?;

        Ok(unwrapped)
    }

    fn delete<T>(store: &Store, name: T) -> Result<(), super::error::Error>
    where
        T: AsRef<str>,
    {
        store.delete_entity(Self::KIND, name.as_ref())?;

        Ok(())
    }

    fn list(store: &Store) -> Result<Vec<Self::Type>, super::error::Error> {
        let entities = store.get_kind(Self::KIND)?;

        let mut vms = vec![];
        for entity in entities {
            let entity = Self::migrate_version(entity)?;
            let vm = serde_json::value::from_value(entity)?;
            vms.push(vm);
        }

        Ok(vms)
    }

    fn create(&self, store: &Store) -> Result<(), super::error::Error>
    where
        Self: Serialize,
    {
        let val = serde_json::value::to_value(self)?;
        store.create_entity(val)?;
        Ok(())
    }
}

pub trait MigratableEntity {
    fn migrate(entity: Value) -> Result<Value, super::error::Error> {
        Ok(entity)
    }
}
