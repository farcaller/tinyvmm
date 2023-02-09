mod error;

use eyre::Context;
use log::{debug, error, info};
use std::{
    collections::HashMap,
    net::{self, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    select,
    sync::{mpsc::Receiver, Mutex},
    task::JoinHandle,
};
use trust_dns_proto::rr::{self, Record};
use trust_dns_server::{
    authority::{Catalog, ZoneType},
    proto::rr::Name,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
    store::in_memory::InMemoryAuthority,
    ServerFuture,
};

use crate::database::{bridge::Bridge, entity::Entity, virtual_machine::VirtualMachine};

use self::error::Error;

struct DnsHandler {
    authority: Arc<Mutex<HashMap<String, Arc<InMemoryAuthority>>>>,
    catalog: Arc<Mutex<Catalog>>,
    runtime_dir: String,
    reconcile_delay: Duration,
}

#[async_trait::async_trait]
impl RequestHandler for DnsHandler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> ResponseInfo {
        let catalog = self.catalog.lock().await;
        catalog.handle_request(request, response_handle).await
    }
}

impl DnsHandler {
    pub fn new(runtime_dir: String, reconcile_delay: Duration) -> Result<Self, Error> {
        let catalog = Catalog::new();

        Ok(DnsHandler {
            authority: Arc::new(Mutex::new(HashMap::new())),
            catalog: Arc::new(Mutex::new(catalog)),
            runtime_dir,
            reconcile_delay,
        })
    }

    pub async fn run(&self) -> JoinHandle<()> {
        let runtime_dir = self.runtime_dir.clone();
        let authority = self.authority.clone();
        let reconcile_delay = self.reconcile_delay;
        let catalog = self.catalog.clone();
        tokio::spawn(async move {
            loop {
                let mut authority_map = authority.lock().await;
                let err =
                    DnsHandler::reconcile(&runtime_dir, &mut authority_map, catalog.clone()).await;
                if let Err(err) = err {
                    error!("failed to reconcile the dns: {}", err);
                }
                tokio::time::sleep(reconcile_delay).await;
            }
        })
    }

    async fn reconcile(
        runtime_dir: &str,
        authority: &mut HashMap<String, Arc<InMemoryAuthority>>,
        catalog: Arc<Mutex<Catalog>>,
    ) -> Result<(), Error> {
        let bridges = Bridge::list(runtime_dir)?;
        let vms = VirtualMachine::list(runtime_dir)?;

        for bridge in bridges {
            let vms = vms
                .iter()
                .filter(|vm| vm.spec.bridge == bridge.metadata.name);

            let origin = Name::from_str(&bridge.spec.dns_zone)?;
            let entry = authority
                .entry(bridge.spec.dns_zone.clone())
                .or_insert_with(|| {
                    Arc::new(InMemoryAuthority::empty(
                        origin.clone(),
                        ZoneType::Primary,
                        false,
                    ))
                });

            {
                let mut catalog = catalog.lock().await;
                catalog.upsert(origin.into(), Box::new(entry.clone()));
            }

            for vm in vms {
                let name =
                    Name::from_str(&format!("{}.{}", vm.metadata.name, bridge.spec.dns_zone))?;
                let record = Record::from_rdata(
                    name.clone(),
                    500,
                    rr::RData::A(net::Ipv4Addr::from_str(&vm.spec.ip)?),
                );

                entry.upsert(record, 0).await;
                // TODO: how do I even clean those?..
            }

            debug!("updated dns zone for {}", bridge.spec.dns_zone);
        }

        Ok(())
    }
}

pub async fn run_server(
    addr: SocketAddr,
    runtime_dir: String,
    reconcile_delay: Duration,
    mut shutdown_signal: Receiver<()>,
) -> eyre::Result<()> {
    info!("starting the dns server");

    let handler = DnsHandler::new(runtime_dir, reconcile_delay)?;

    let _h = handler.run().await;

    let mut server_fut = ServerFuture::new(handler);
    let udp_socket = UdpSocket::bind(addr).await?;
    server_fut.register_socket(udp_socket);

    select! {
        er = server_fut.block_until_done() => {
            return er.wrap_err_with(|| "failed to run the dns server".to_string());
        }

        _ = shutdown_signal.recv() => {
            debug!("shutting down the dns server");
        }
    }

    Ok(())
}
