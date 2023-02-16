mod error;

use eyre::Context;
use log::{debug, error, info};
use std::{
    collections::HashMap,
    net::{self, SocketAddr},
    str::FromStr,
    sync::Arc,
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

use crate::database::{
    bridge::Bridge, entity::Entity, store::Store, virtual_machine::VirtualMachine,
};

use self::error::Error;

struct DnsHandler {
    authority: Arc<Mutex<HashMap<String, Arc<InMemoryAuthority>>>>,
    catalog: Arc<Mutex<Catalog>>,
    store: Store,
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
    pub fn new(store: Store) -> Result<Self, Error> {
        let catalog = Catalog::new();

        Ok(DnsHandler {
            authority: Arc::new(Mutex::new(HashMap::new())),
            catalog: Arc::new(Mutex::new(catalog)),
            store,
        })
    }

    pub async fn run(&self) -> eyre::Result<JoinHandle<()>> {
        let authority = self.authority.clone();
        let catalog = self.catalog.clone();
        let store = self.store.clone();

        Ok(tokio::spawn(async move {
            let mut subscriber = store.watch_entities("/");
            {
                let mut authority_map = authority.lock().await;
                let _err = DnsHandler::reconcile(&store, &mut authority_map, catalog.clone()).await;
            }

            while let Some(event) = (&mut subscriber).await {
                debug!(
                    "dns reconciler event: {}",
                    String::from_utf8(event.key().to_ascii_lowercase())
                        .or_else(|_| Ok::<String, ()>("?".into()))
                        .unwrap()
                );
                let mut authority_map = authority.lock().await;
                let err = DnsHandler::reconcile(&store, &mut authority_map, catalog.clone()).await;
                if let Err(err) = err {
                    error!("failed to reconcile the dns: {}", err);
                }
            }
        }))
    }

    async fn reconcile(
        store: &Store,
        authority: &mut HashMap<String, Arc<InMemoryAuthority>>,
        catalog: Arc<Mutex<Catalog>>,
    ) -> Result<(), Error> {
        let bridges = Bridge::list(store)?;
        let vms = VirtualMachine::list(store)?;

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
    store: Store,
    mut shutdown_signal: Receiver<()>,
) -> eyre::Result<()> {
    info!("starting the dns server");

    let handler = DnsHandler::new(store)?;

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
