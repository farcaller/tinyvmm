mod bridges;
mod virtualmachines;

use std::time::Duration;

use log::{debug, info, warn};
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
    time,
};

pub struct Config {
    pub reconcile_delay: Duration,
    pub shutdown_signal: Receiver<()>,
    pub runtime_dir: String,
    pub dns_listener: String,
}

async fn reconcile(runtime_dir: &str, dns_listener: &str) {
    info!("reconciling vm units");
    let res = virtualmachines::reconcile(runtime_dir).await;
    if let Err(e) = res {
        warn!("failed reconciling vm units: {}", e);
    }
    info!("reconciling bridges");
    let res = bridges::reconcile(runtime_dir, dns_listener).await;
    if let Err(e) = res {
        warn!("failed reconciling bridges: {}", e);
    }
}

pub async fn main(mut config: Config, shutdown: Sender<eyre::Result<()>>) {
    info!("starting the unit reconciler");
    loop {
        reconcile(&config.runtime_dir, &config.dns_listener).await;
        select! {
            _ = time::sleep(config.reconcile_delay) => {}
            _ = config.shutdown_signal.recv() => {
                debug!("shutting down the unit reconciler");
                shutdown.send(Ok(())).await.unwrap();
                return;
            }
        }
    }
}
