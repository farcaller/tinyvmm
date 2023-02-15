use super::error::Error;
use serde_json::value::Value;

pub trait EntityObject {
    fn as_map(&self) -> Result<&serde_json::map::Map<String, Value>, Error>;
    fn is_version(&self, version: &str) -> Result<bool, Error> {
        Ok(self.as_map()?.get_existing("apiVersion")? == version)
    }
}

pub trait ValueGetter {
    fn get_existing(&self, index: &'static str) -> Result<&Value, Error>;
}

impl EntityObject for Value {
    fn as_map(&self) -> Result<&serde_json::map::Map<String, Value>, Error> {
        self.as_object().ok_or(Error::NotAnObject)
    }
}

impl ValueGetter for Value {
    fn get_existing(&self, index: &'static str) -> Result<&Value, Error> {
        self.get(index).ok_or(Error::MissingKey(index))
    }
}

impl ValueGetter for serde_json::map::Map<String, Value> {
    fn get_existing(&self, index: &'static str) -> Result<&Value, Error> {
        self.get(index).ok_or(Error::MissingKey(index))
    }
}
