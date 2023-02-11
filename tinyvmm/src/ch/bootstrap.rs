use std::{path::PathBuf, time::Duration};

use backoff::ExponentialBackoffBuilder;
use byte_unit::Byte;
use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, Uri};
use net_util::MacAddr;
use vmm::vm_config::{
    ConsoleConfig, ConsoleOutputMode, CpusConfig, DiskConfig, MemoryConfig, NetConfig,
    PayloadConfig, RngConfig, VmConfig,
};

use crate::database::{entity::Entity, virtual_machine::VirtualMachine};

use super::{error::Error, get_vm_tap_name};

const HYPERVISOR_FW: &str = "hypervisor";

pub async fn bootstrap_vm(runtime_dir: &str, name: &str) -> Result<(), Error> {
    let vm = VirtualMachine::get(runtime_dir, name)?;

    let params = VmConfig {
        cpus: CpusConfig {
            boot_vcpus: vm.spec.cpus,
            max_vcpus: vm.spec.cpus,
            ..Default::default()
        },
        memory: MemoryConfig {
            // TODO: fix the memory parsing in the deserializer so that the number is always correct in here
            size: Byte::from_str(format!("{}iB", vm.spec.memory))?.get_bytes() as u64,
            ..Default::default()
        },
        payload: Some(PayloadConfig {
            kernel: Some(PathBuf::from(runtime_dir).join(HYPERVISOR_FW)),
            ..Default::default()
        }),
        disks: Some(vec![DiskConfig {
            path: Some(PathBuf::from(vm.spec.disk)),
            ..Default::default()
        }]),
        net: Some(vec![NetConfig {
            tap: Some(get_vm_tap_name(name)),
            mac: MacAddr::parse_str(&vm.spec.mac)?,
            ..Default::default()
        }]),
        rng: RngConfig::default(),
        balloon: None,
        fs: None,
        pmem: None,
        serial: ConsoleConfig {
            file: Some(
                PathBuf::from("/run")
                    .join(format!("tinyvmi-{}", name))
                    .join("serial"),
            ),
            mode: ConsoleOutputMode::File,
            iommu: false,
        },
        console: ConsoleConfig {
            file: Some(
                PathBuf::from("/run")
                    .join(format!("tinyvmi-{}", name))
                    .join("console"),
            ),
            mode: ConsoleOutputMode::File,
            iommu: false,
        },
        devices: None,
        user_devices: None,
        vdpa: None,
        vsock: None,
        iommu: false,
        sgx_epc: None,
        numa: None,
        watchdog: false,
        platform: None, // TODO: uuid & serial
        tpm: None,
    };

    let request_op = || async {
        let url = Uri::new(
            PathBuf::from("/run")
                .join(format!("tinyvmi-{}", name))
                .join("api.sock"),
            "/api/v1/vm.create",
        );

        let req = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("host", "localhost")
            .header("accept", "*/*")
            .body(Body::from(
                serde_json::to_string(&params).map_err(Error::Serialize)?,
            ))
            .map_err(Error::Http)?;

        let client = Client::unix();

        Ok(client.request(req).await.map_err(Error::Hyper)?)
    };

    let backoff = ExponentialBackoffBuilder::new()
        .with_max_elapsed_time(Some(Duration::from_secs(60)))
        .build();
    let response = backoff::future::retry(backoff, request_op).await?;

    let (parts, body) = response.into_parts();

    if !parts.status.is_success() {
        let bytes = &hyper::body::to_bytes(body).await?;
        let rep = (std::str::from_utf8(bytes)?).to_string();
        return Err(Error::HttpNoSuccess(parts.status.as_u16(), rep));
    }

    Ok(())
}
