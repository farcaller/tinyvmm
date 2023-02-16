mod bridges;
mod virtualmachines;

use std::time::Duration;

use log::{debug, info, warn};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
    time,
};

use crate::database::store::Store;

pub struct Config {
    pub reconcile_delay: Duration,
    pub shutdown_signal: Receiver<()>,
    pub store: Store,
    pub dns_listener: String,
}

async fn reconcile(store: &Store, dns_listener: &str) {
    info!("reconciling vm units");
    let res = virtualmachines::reconcile(store).await;
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

    loop {
        reconcile(&config.store, &config.dns_listener).await;
        select! {
            _ = time::sleep(config.reconcile_delay) => {}
            _ = config.shutdown_signal.recv() => {
                debug!("shutting down the unit reconciler");
                shutdown.send(Ok(())).await.unwrap();
                return Ok(());
            }
        }
    }
}
