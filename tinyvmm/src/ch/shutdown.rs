use std::path::PathBuf;

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, Uri};

use crate::database::virtual_machine::VirtualMachine;

use super::{error::Error, get_vm_tap_name};

const DATA_DIR: &str = "/var/lib/tinyvmm";
const RUN_DIR: &str = "/run/tinyvmm/vms";
const HYPERVISOR_FW: &str = "hypervisor";
pub async fn shutdown_vm(name: &str) -> Result<(), Error> {
    let vm = VirtualMachine::get(name)?;

    let url = Uri::new(
        PathBuf::from(RUN_DIR).join(format!("{}.sock", name)),
        "/api/v1/vm.power-button",
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
