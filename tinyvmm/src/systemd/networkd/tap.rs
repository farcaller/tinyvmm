use handlebars::Handlebars;
use indoc::indoc;
use serde_json::json;

use super::create_and_start_unit;

use crate::systemd::error::SystemdUnitCreationError;

pub async fn create_tap_network(
    name: &str,
    bridge: &str,
    mac: &str,
) -> Result<(), SystemdUnitCreationError> {
    let ini = Handlebars::new().render_template(
        indoc! {"
            [Match]
            Name={{name}}

            [Network]
            Bridge={{bridge}}

            [Link]
            MACAddress={{mac}}
        "},
        &json!({
            "name": name,
            "bridge": bridge,
            "mac": vm_mac_to_tap_mac(mac),
        }),
    )?;

    return create_and_start_unit(name, "network", &ini).await;
}

pub async fn create_tap(name: &str, mac: &str) -> Result<(), SystemdUnitCreationError> {
    let ini = Handlebars::new().render_template(
        indoc! {"
            [NetDev]
            Name={{name}}
            Kind=tap
            # MACAddress={{mac}}
            "},
        &json!({
            "name": name,
            "mac": vm_mac_to_tap_mac(mac),
        }),
    )?;

    return create_and_start_unit(name, "netdev", &ini).await;
}

fn vm_mac_to_tap_mac(vm_mac: &str) -> String {
    // TODO: this needs a better solution to avoid confusion
    let stable_part = &vm_mac[3..];
    format!("76:{stable_part}")
}
