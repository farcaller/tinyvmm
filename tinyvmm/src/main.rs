mod apiserver;
mod ch;
mod cli;
pub mod client;
mod database;
mod dbus;
mod dns;
mod systemd;
mod unitserver;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    cli::main().await
}
