use std::path::PathBuf;

use serde_json::Value;
use sled::Tree;

use super::{
    error::Error,
    serde::{EntityObject, ValueGetter},
};

#[derive(Clone)]
pub struct Store {
    entity_tree: Tree,
    pub store_path: PathBuf,
}

impl Store {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        let db = sled::open(path)?;

        let entity_tree = db.open_tree("entities")?;

        Ok(Store {
            entity_tree,
            store_path: path.into(),
        })
    }

    fn key(kind: &str, name: &str) -> String {
        format!("/{kind}/{name}")
    }

    pub fn watch_entities(&self, prefix: &str) -> sled::Subscriber {
        self.entity_tree.watch_prefix(prefix)
    }

    pub fn get_entity(&self, kind: &str, name: &str) -> Result<Option<Value>, Error> {
        let key = Self::key(kind, name);
        let entity = self.entity_tree.get(key)?;

        match entity {
            Some(bytes) => Ok(Some(serde_json::from_slice(bytes.as_ref())?)),
            None => Ok(None),
        }
    }

    pub fn delete_entity(&self, kind: &str, name: &str) -> Result<(), Error> {
        let key = Self::key(kind, name);
        self.entity_tree.remove(key)?;

        Ok(())
    }

    pub fn get_kind(&self, kind: &str) -> Result<Vec<Value>, Error> {
        let start_key = format!("/{kind}/");
        let end_key = format!("/{kind}0");

        self.entity_tree
            .range(start_key..end_key)
            .map(|e| {
                e.map_err(Error::SledError).and_then(|bytes| {
                    serde_json::from_slice(bytes.1.as_ref()).map_err(Error::Serialize)
                })
            })
            .collect()
    }

    pub fn create_entity(&self, e: Value) -> Result<(), Error> {
        let kind = e
            .get_existing("kind")?
            .as_str()
            .ok_or(Error::MissingKey("kind"))?;
        let name = e
            .get_existing("metadata")?
            .as_map()?
            .get_existing("name")?
            .as_str()
            .ok_or(Error::MissingKey("name"))?;
        let key = Self::key(kind, name);
        let data = serde_json::to_string(&e)?;

        let result =
            self.entity_tree
                .compare_and_swap(key, None as Option<&[u8]>, Some(data.as_bytes()));
        let result = result?;

        result.map_err(|_| Error::KeyExists {
            kind: kind.into(),
            name: name.into(),
        })
    }
}
