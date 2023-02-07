use std::path::PathBuf;

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, Uri};

use crate::database::{entity::Entity, virtual_machine::VirtualMachine};

use super::error::Error;

pub async fn start_vm(runtime_dir: &str, name: &str) -> Result<(), Error> {
    let _vm = VirtualMachine::get(runtime_dir, name)?;

    let url = Uri::new(
        PathBuf::from("/run")
            .join(format!("tinyvmi-{}", name))
            .join("api.sock"),
        "/api/v1/vm.boot",
    );
    let client = Client::unix();

    let req = Request::builder()
        .method(Method::PUT)
        .uri(url)
        .header("host", "localhost")
        .header("accept", "*/*")
        .body(Body::empty())?;

    let response = client.request(req).await?;
    let (parts, body) = response.into_parts();

    if !parts.status.is_success() {
        let bytes = &hyper::body::to_bytes(body).await?;
        let rep = (std::str::from_utf8(bytes)?).to_string();
        return Err(Error::HttpNoSuccess(parts.status.as_u16(), rep));
    }

    Ok(())
}

pub async fn shutdown_vm(runtime_dir: &str, name: &str) -> Result<(), Error> {
    let _vm = VirtualMachine::get(runtime_dir, name)?;

    let api_path = PathBuf::from("/run")
        .join(format!("tinyvmi-{}", name))
        .join("api.sock");

    let url = Uri::new(&api_path, "/api/v1/vm.power-button");
    let client = Client::unix();

    let req = Request::builder()
        .method(Method::PUT)
        .uri(url)
        .header("host", "localhost")
        .header("accept", "*/*")
        .body(Body::empty())?;

    let response = client.request(req).await?;
    let (parts, body) = response.into_parts();

    if !parts.status.is_success() {
        let bytes = &hyper::body::to_bytes(body).await?;
        let rep = (std::str::from_utf8(bytes)?).to_string();
        return Err(Error::HttpNoSuccess(parts.status.as_u16(), rep));
    }

    for _ in 0..240 {
        if !api_path.exists() {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}
