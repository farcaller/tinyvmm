use thiserror::Error;

use crate::database;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] database::error::Error),

    #[error("dns error: {0}")]
    Dns(#[from] trust_dns_proto::error::ProtoError),

    #[error("ip address parse error: {0}")]
    IpAddressParse(#[from] std::net::AddrParseError),
}
