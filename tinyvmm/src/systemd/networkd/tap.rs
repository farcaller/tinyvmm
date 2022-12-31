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
            "mac": mac,
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
            "mac": mac,
        }),
    )?;

    return create_and_start_unit(name, "netdev", &ini).await;
}
