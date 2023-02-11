use handlebars::Handlebars;
use indoc::indoc;
use serde::Serialize;
use serde_json::json;

use crate::systemd::error::SystemdUnitCreationError::{self};

use super::create_and_start_unit;

pub async fn create_bridge(name: &str) -> Result<(), SystemdUnitCreationError> {
    let ini = Handlebars::new().render_template(
        indoc! {"
            [NetDev]
            Name={{name}}
            Kind=bridge
            "},
        &json!({
            "name": name,
        }),
    )?;

    return create_and_start_unit(name, "netdev", &ini).await;
}

#[derive(Serialize, Debug)]
pub struct Lease {
    pub mac: String,
    pub ip: String,
}

pub async fn create_bridge_network(
    name: &str,
    domain: &str,
    address: &ipnet::Ipv4Net,
    dns_listener: &str,
    dns_server: &str,
    leases: Vec<Lease>,
) -> Result<(), SystemdUnitCreationError> {
    let ini = Handlebars::new().render_template(
        indoc! {"
            [Match]
            Name={{name}}

            [Network]
            Address={{address}}
            DNS={{dns}}
            Domains={{domains}}
            ConfigureWithoutCarrier=yes
            IgnoreCarrierLoss=yes
            DHCPServer=yes

            [DHCPServer]
            EmitDNS=yes
            DNS={{dns_server}}
            EmitRouter=yes
            Router={{router}}

            {{#each leases as |l|}}
            [DHCPServerStaticLease]
            MACAddress={{l.mac}}
            Address={{l.ip}}
            {{/each}}
        "},
        &json!({
            "name": name,
            "address": format!("{}", address),
            "dns": dns_listener,
            "dns_server": dns_server,
            "router": address.addr(),
            "domains": format!("~{}", domain),
            "leases": leases,
        }),
    )?;

    return create_and_start_unit(name, "network", &ini).await;
}
