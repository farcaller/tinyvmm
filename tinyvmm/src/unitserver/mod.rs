mod bridges;
mod virtualmachines;

use log::{debug, info, warn};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::database::store::Store;

pub struct Config {
    pub shutdown_signal: Receiver<()>,
    pub store: Store,
    pub dns_listener: String,
    pub api_server: String,
}

async fn reconcile(store: &Store, dns_listener: &str, api_server: &str) {
    info!("reconciling vm units");
    let res = virtualmachines::reconcile(store, api_server).await;
    if let Err(e) = res {
        warn!("failed reconciling vm units: {}", e);
    }
    info!("reconciling bridges");
    let res = bridges::reconcile(store, dns_listener).await;
    if let Err(e) = res {
        warn!("failed reconciling bridges: {}", e);
    }
}

pub async fn main(mut config: Config, shutdown: Sender<eyre::Result<()>>) -> eyre::Result<()> {
    info!("starting the unit reconciler");

    let mut subscriber = config.store.watch_entities("/");

    tokio::spawn(async move {
        reconcile(&config.store, &config.dns_listener, &config.api_server).await;
        while let Some(event) = (&mut subscriber).await {
            debug!(
                "unit reconciler event: {}",
                String::from_utf8(event.key().to_ascii_lowercase())
                    .or_else(|_| Ok::<String, ()>("?".into()))
                    .unwrap()
            );
            reconcile(&config.store, &config.dns_listener, &config.api_server).await;
        }
    });

    config.shutdown_signal.recv().await;
    debug!("shutting down the unit reconciler");
    shutdown.send(Ok(())).await.unwrap();

    Ok(())
}
