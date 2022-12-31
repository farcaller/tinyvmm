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
}

async fn reconcile() {
    info!("reconciling vm units");
    let res = virtualmachines::reconcile().await;
    if let Err(e) = res {
        warn!("failed reconciling vm units: {}", e);
    }
    info!("reconciling bridges");
    let res = bridges::reconcile().await;
    if let Err(e) = res {
        warn!("failed reconciling bridges: {}", e);
    }
}

pub async fn main(mut config: Config, shutdown: Sender<eyre::Result<()>>) {
    info!("starting the unit reconciler");
    loop {
        reconcile().await;
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
