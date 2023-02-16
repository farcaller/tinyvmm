use std::path::PathBuf;

use hyper::{Body, Method, Request};
use hyperlocal::{UnixClientExt, Uri};

use crate::database::virtual_machine::VirtualMachine;

use self::error::Error;

mod error;

pub struct Client {
    api_server: String,
}

impl Client {
    pub fn new(api_server: String) -> Self {
        Client { api_server }
    }

    pub fn virtualmachines(&self) -> VirtualMachineClient {
        VirtualMachineClient {
            api_server: self.api_server.clone(),
        }
    }
}

pub struct VirtualMachineClient {
    api_server: String,
}

impl VirtualMachineClient {
    async fn http_get(url: Uri) -> Result<String, Error> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(url)
            .header("host", "localhost")
            .header("accept", "application/json")
            .body(Body::empty())?;

        let client = hyper::Client::unix();

        let response = client.request(req).await.map_err(Error::Hyper)?;

        let (parts, body) = response.into_parts();

        let bytes = &hyper::body::to_bytes(body).await?;
        let rep = (std::str::from_utf8(bytes)?).to_string();

        if !parts.status.is_success() {
            return Err(Error::HttpNoSuccess(parts.status.as_u16(), rep));
        }

        Ok(rep)
    }

    pub async fn list(&self) -> Result<VirtualMachine, Error> {
        let url = Uri::new(
            PathBuf::from(self.api_server.clone()),
            "/api/v1/virtualmachines",
        );

        Ok(serde_json::from_str(&Self::http_get(url).await?)?)
    }

    pub async fn get(&self, name: &str) -> Result<VirtualMachine, Error> {
        let url = Uri::new(
            PathBuf::from(self.api_server.clone()),
            &format!("/api/v1/virtualmachines/{name}"),
        );

        Ok(serde_json::from_str(&Self::http_get(url).await?)?)
    }
}
