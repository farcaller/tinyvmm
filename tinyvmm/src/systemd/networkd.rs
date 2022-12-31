pub(crate) mod bridge;
pub(crate) mod tap;

use std::{fs, path::Path};
use zbus::Connection;

use super::error::SystemdUnitCreationError::{self, *};

use crate::dbus::networkd::NetworkdProxy;

const RUNTIME_NETWORK_DIR: &str = "/run/systemd/network";

pub(crate) async fn create_and_start_unit(
    name: &str,
    unit_type: &str,
    body: &str,
) -> Result<(), SystemdUnitCreationError> {
    let runtime_dir = Path::new(RUNTIME_NETWORK_DIR);
    if !runtime_dir.is_dir() {
        fs::create_dir(runtime_dir).map_err(SystemdUnitCreationError::CannotCreateRuntimeDir)?;
    }
    let unit_name = format!("{}.{}", name, unit_type);
    fs::write(runtime_dir.join(unit_name), body)
        .map_err(SystemdUnitCreationError::CannotCreateUnitFile)?;

    let connection = Connection::system().await?;
    let proxy = NetworkdProxy::new(&connection).await?;

    proxy.reload().await?;

    Ok(())
}

pub async fn destroy_netdev(name: &str) -> Result<(), SystemdUnitCreationError> {
    use futures::stream::TryStreamExt;

    let runtime_dir = Path::new(RUNTIME_NETWORK_DIR);
    if !runtime_dir.is_dir() {
        return Ok(());
    }
    let netdev = runtime_dir.join(format!("{}.netdev", name));
    let network = runtime_dir.join(format!("{}.network", name));

    if network.is_file() {
        fs::remove_file(&network).map_err(|e| CannotRemoveUnitFile(network, e))?;
    }
    if netdev.is_file() {
        fs::remove_file(&netdev).map_err(|e| CannotRemoveUnitFile(netdev, e))?;
    }

    let connection = Connection::system().await?;
    let proxy = NetworkdProxy::new(&connection).await?;

    proxy.reload().await?;

    let (connection, handle, _) = rtnetlink::new_connection().map_err(NetlinkConnection)?;
    // TODO: does this leak?
    tokio::spawn(connection);
    let mut links = handle.link().get().match_name(name.into()).execute();
    if let Some(link) = links.try_next().await? {
        handle.link().del(link.header.index).execute().await?;
        Ok(())
    } else {
        Err(LinkNotFound(name.into()))
    }
}
