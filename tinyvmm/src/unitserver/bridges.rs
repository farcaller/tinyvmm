use crate::{
    database::{bridge::Bridge, entity::Entity, store::Store, virtual_machine::VirtualMachine},
    systemd::bridge::{create_bridge, create_bridge_network, Lease},
};

pub async fn reconcile(store: &Store, dns_listener: &str) -> eyre::Result<()> {
    let bridges = Bridge::list(store)?;
    let vms = VirtualMachine::list(store)?;

    // generate the units for existing bridges and check the diffs
    // if there are any diffs, commit, daemon-reload and start them
    // TODO: actually diff
    for bridge in bridges {
        let name = &bridge.metadata.name;

        let vms = vms.iter().filter(|vm| &vm.spec.bridge == name);

        create_bridge(name).await?;

        create_bridge_network(
            name,
            &bridge.spec.dns_zone,
            &bridge.spec.address.parse().unwrap(),
            dns_listener,
            &bridge.spec.dns_server,
            vms.map(|vm| Lease {
                mac: vm.spec.mac.clone(),
                ip: vm.spec.ip.clone(),
            })
            .collect(),
        )
        .await?;
    }

    // TODO: cleanup and daemon-reload

    Ok(())
}
