use log::{debug, info, trace, warn};

use crate::{
    database::{entity::Entity, virtual_machine::VirtualMachine},
    systemd,
};

pub async fn reconcile(runtime_dir: &str) -> eyre::Result<()> {
    let self_exe = &std::env::args().next().unwrap();

    let vms = VirtualMachine::list(runtime_dir)?;

    debug!("got {} vms to reconcile", vms.len());

    // generate the units for existing vms and check the diffs
    // if there are any diffs, commit, daemon-reload and start them
    for vm in vms {
        let name = &vm.metadata.name;
        let bridge_name = &vm.spec.bridge;

        debug!("reconciling vm {name}");

        let has_diffs = systemd::has_diffs(name, bridge_name, runtime_dir, self_exe).await;
        debug!("vm {name} diffs: {has_diffs:?}");
        match has_diffs {
            Err(e) => {
                info!("failed to check for diffs for {}: {}", name, e);
                info!("will try to reconcile");
                if let Err(e) =
                    systemd::create_vm_service(name, bridge_name, runtime_dir, self_exe).await
                {
                    warn!("systemd::create_vm_service failed for {}: {}", name, e);
                }
                if let Err(e) = systemd::start_service(name).await {
                    warn!("systemd::start_service failed for {}: {}", name, e);
                }
                debug!("reconciled and started {}", name);
            }
            Ok(true) => {
                info!("{} changed, will try to reconcile", name);
                if let Err(e) =
                    systemd::create_vm_service(name, bridge_name, runtime_dir, self_exe).await
                {
                    warn!("systemd::create_vm_service failed for {}: {}", name, e);
                }
                if let Err(e) = systemd::start_service(name).await {
                    warn!("systemd::start_service failed for {}: {}", name, e);
                }
                debug!("reconciled and started {}", name);
            }
            Ok(false) => {
                trace!("already reconciled {}", name);
            }
        }
    }

    // list all tinyvmi units, stop and cleanup nonexistent ones. Daemon-reload after

    Ok(())
}
