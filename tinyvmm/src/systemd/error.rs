use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SystemdUnitCreationError {
    #[error("failed to create the runtime config directory: {0}")]
    CannotCreateRuntimeDir(#[source] io::Error),

    #[error("failed to create the unit file: {0}")]
    CannotCreateUnitFile(#[source] io::Error),

    #[error("failed to read the unit file: {0}")]
    CannotReadUnitFile(#[source] io::Error),

    #[error("failed to remove the unit file {0}: {1}")]
    CannotRemoveUnitFile(PathBuf, #[source] io::Error),

    #[error("netlink connection error: {0}")]
    NetlinkConnection(#[source] io::Error),

    #[error("netlink error: {0}")]
    Netlink(#[from] rtnetlink::Error),

    #[error("link {0} not found")]
    LinkNotFound(String),

    #[error("dbus error: {0}")]
    Zbus(#[from] zbus::Error),

    #[error("template render error: {0}")]
    Handlebars(#[from] handlebars::RenderError),
}
