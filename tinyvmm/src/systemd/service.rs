use handlebars::Handlebars;
use indoc::indoc;
use log::trace;
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use zbus::Connection;

use super::error::SystemdUnitCreationError;

use crate::dbus::systemd::SystemdProxy;

const RUNTIME_NETWORK_DIR: &str = "/run/systemd/system";

fn get_unit_path(name: &str) -> PathBuf {
    let unit_name = format!("{}.service", name);

    Path::new(RUNTIME_NETWORK_DIR).join(unit_name)
}

async fn create_and_start_systemd_unit(
    name: &str,
    body: &str,
) -> Result<(), SystemdUnitCreationError> {
    let runtime_dir = Path::new(RUNTIME_NETWORK_DIR);
    if !runtime_dir.is_dir() {
        panic!("no systemd runtime dir!");
    }
    let unit_name = format!("{}.service", name);
    fs::write(get_unit_path(name), body).map_err(SystemdUnitCreationError::CannotCreateUnitFile)?;

    let connection = Connection::system().await?;
    let proxy = SystemdProxy::new(&connection).await?;

    proxy.load_unit(&unit_name).await?;

    Ok(())
}

pub async fn create_vm_service(
    name: &str,
    bridge_name: &str,
    self_exe: &str,
) -> Result<(), SystemdUnitCreationError> {
    let units = generate_vm_service(name, bridge_name, self_exe).await?;
    for (name, config) in units {
        create_and_start_systemd_unit(&name, &config).await?;
    }

    Ok(())
}

pub async fn start_service(name: &str) -> Result<(), SystemdUnitCreationError> {
    let connection = Connection::system().await?;
    let proxy = SystemdProxy::new(&connection).await?;

    let unit_name = format!("{}.service", get_systemd_unit_name(name));
    trace!("starting {}", unit_name);
    proxy.start_unit(&unit_name, "replace").await?;

    Ok(())
}

pub async fn has_diffs(
    name: &str,
    bridge_name: &str,
    self_exe: &str,
) -> Result<bool, SystemdUnitCreationError> {
    let units = generate_vm_service(name, bridge_name, self_exe).await?;
    for (name, config) in units {
        let path = get_unit_path(&name);
        let contents =
            fs::read_to_string(path).map_err(SystemdUnitCreationError::CannotReadUnitFile)?;
        if contents != config {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn generate_vm_service(
    name: &str,
    bridge_name: &str,
    self_exe: &str,
) -> Result<HashMap<String, String>, SystemdUnitCreationError> {
    let mut res = HashMap::new();

    let ini = Handlebars::new().render_template(
        indoc! {r#"
            [Unit]
            Requires=sys-subsystem-net-devices-{{netbr}}.device
            Requires={{netdev}}.service
            After=sys-subsystem-net-devices-{{netbr}}.device
            After={{netdev}}.service

            [Service]
            Type=simple
            ExecStart=/run/wrappers/bin/cloud-hypervisor --api-socket=${RUNTIME_DIRECTORY}/api.sock -v

            ExecStartPost={{self_exe}} systemd bootstrap-post {{name}}
            ExecStartPost={{self_exe}} start {{name}}

            ExecStop={{self_exe}} stop {{name}}

            RuntimeDirectory=tinyvmi-{{name}}
            "#},
        &json!({
            "name": name,
            "self_exe": format!("{}", std::fs::canonicalize(self_exe).unwrap().to_string_lossy()),
            "netbr": bridge_name,
            "netdev": get_systemd_tap_unit_name(name),
        }),
    )?;

    res.insert(get_systemd_unit_name(name), ini);

    let ini = Handlebars::new().render_template(
        indoc! {"
            [Unit]
            # Requires=sys-subsystem-net-devices-{{netbr}}.device
            # After=sys-subsystem-net-devices-{{netbr}}.device
            PartOf={{vmservice}}.service

            [Service]
            Type=oneshot
            RemainAfterExit=yes
            ExecStart={{self_exe}} systemd bootstrap-pre {{name}}
            ExecStop={{self_exe}} systemd teardown {{name}}
            "},
        &json!({
            "name": name,
            "self_exe": format!("{}", std::fs::canonicalize(self_exe).unwrap().to_string_lossy()),
            "netbr": bridge_name,
            "vmservice": get_systemd_unit_name(name),
        }),
    )?;

    res.insert(get_systemd_tap_unit_name(name), ini);

    Ok(res)
}

pub fn get_systemd_unit_name(name: &str) -> String {
    format!("tinyvmi-{}", name)
}

pub fn get_systemd_tap_unit_name(name: &str) -> String {
    format!("tinyvmi-tap-{}", name)
}
