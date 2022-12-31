use std::time::Duration;

use crate::{
    self as tvm,
    database::{entity::Entity, virtual_machine::VirtualMachine},
};
use clap::{Parser, Subcommand};
use log::debug;
use tokio::{signal, sync::mpsc};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Internal {
        #[command(subcommand)]
        command: InternalCommands,
    },
    Systemd {
        #[command(subcommand)]
        command: SystemdCommands,
    },
    Start {
        name: String,
    },
    Stop {
        name: String,
    },
    ApiServer {
        #[clap(long)]
        listen: String,
    },
    UnitServer {
        #[clap(long)]
        reconcile_delay: u64,
    },
    Serve {
        #[clap(long)]
        listen: String,
        #[clap(long)]
        reconcile_delay: u64,
    },
}

#[derive(Debug, Subcommand)]
enum SystemdCommands {
    BootstrapPre { name: String },
    BootstrapPost { name: String },
    Teardown { name: String },
}

#[derive(Debug, Subcommand)]
enum BridgeCommands {
    Create {
        name: String,
        #[clap(long)]
        address: ipnet::Ipv4Net,
        #[clap(long)]
        dns_zone: String,
    },
    Destroy {
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum TapCommands {
    Create {
        name: String,
        #[clap(long)]
        mac: String,
        #[clap(long)]
        bridge: String,
    },
    Destroy {
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum InternalCommands {
    Bridge {
        #[command(subcommand)]
        command: BridgeCommands,
    },
    Tap {
        #[command(subcommand)]
        command: TapCommands,
    },
    CreateVMService {
        name: String,
    },
    Networkd {
        #[command(subcommand)]
        command: NetworkdCommand,
    },
}

#[derive(Debug, Subcommand)]
enum NetworkdCommand {
    Reload,
    Describe { name: String },
}

async fn internal_command(cmd: &InternalCommands) -> eyre::Result<()> {
    use BridgeCommands as br;
    use InternalCommands::*;
    use TapCommands as tap;
    match cmd {
        Bridge {
            command:
                br::Create {
                    name,
                    address,
                    dns_zone,
                },
        } => {
            let vms = VirtualMachine::list()?;
            let mut leases = vec![];
            for vm in vms {
                leases.push(tvm::systemd::bridge::Lease {
                    mac: vm.spec.mac,
                    ip: vm.spec.ip,
                });
            }
            tvm::systemd::bridge::create_bridge(name).await?;
            tvm::systemd::bridge::create_bridge_network(name, dns_zone, address, leases).await?;
        }
        Bridge {
            command: br::Destroy { name },
        } => {
            tvm::systemd::destroy_netdev(name).await?;
        }
        Tap {
            command: tap::Create { name, mac, bridge },
        } => {
            let tap_name = tvm::ch::get_vm_tap_name(name);
            tvm::systemd::tap::create_tap(&tap_name, mac).await?;
            tvm::systemd::tap::create_tap_network(&tap_name, bridge, mac).await?;
        }
        Tap {
            command: tap::Destroy { name },
        } => {
            tvm::systemd::destroy_netdev(name).await?;
        }
        CreateVMService { name } => {
            tvm::systemd::create_vm_service(name, &std::env::args().next().unwrap()).await?;
        }
        Networkd { command } => networkd_command(command).await?,
    }
    Ok(())
}

async fn systemd_command(cmd: &SystemdCommands) -> eyre::Result<()> {
    use SystemdCommands::*;
    match cmd {
        BootstrapPre { name } => {
            let tap_name = tvm::ch::get_vm_tap_name(name);

            let vm = VirtualMachine::get(name)?;

            tvm::systemd::tap::create_tap(&tap_name, &vm.spec.mac).await?;
            tvm::systemd::tap::create_tap_network(&tap_name, "tvbr0", &vm.spec.mac).await?;
        }
        BootstrapPost { name } => tvm::ch::bootstrap::bootstrap_vm(name).await?,
        Teardown { name } => tvm::systemd::destroy_netdev(&tvm::ch::get_vm_tap_name(name)).await?,
    }
    Ok(())
}
async fn networkd_command(cmd: &NetworkdCommand) -> eyre::Result<()> {
    use crate::dbus::networkd::NetworkdProxy;
    use zbus::Connection;
    use NetworkdCommand::*;

    let connection = Connection::system().await?;

    let proxy = NetworkdProxy::new(&connection).await?;

    match cmd {
        Reload => {
            proxy.reload().await?;
            Ok(())
        }
        Describe { name } => {
            let (ifindex, _path) = proxy.get_link_by_name(name).await?;
            debug!("interface index={}", ifindex);
            let json = proxy.describe_link(ifindex).await?;

            println!("{}", json);

            Ok(())
        }
    }
}

async fn start_vm(name: &str) -> eyre::Result<()> {
    tvm::ch::runtime::start_vm(name).await?;
    Ok(())
}

async fn stop_vm(name: &str) -> eyre::Result<()> {
    tvm::ch::runtime::shutdown_vm(name).await?;
    Ok(())
}

async fn run_apiserver(listen: &str) -> eyre::Result<()> {
    tvm::apiserver::run_server(listen).await?;
    Ok(())
}

async fn run_unitserver(reconcile_delay: u64) -> eyre::Result<()> {
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);
    let (terminated_send, mut terminated_recv) = mpsc::channel(1);

    let config = tvm::unitserver::Config {
        reconcile_delay: Duration::from_secs(reconcile_delay),
        shutdown_signal: shutdown_recv,
    };

    let worker = tvm::unitserver::main(config, terminated_send);
    let handle = tokio::spawn(worker);
    let shutdown = signal::ctrl_c();

    tokio::select! {
        _ = shutdown => {
            debug!("SIGINT received; starting forced shutdown");
            shutdown_send.send(()).await?;
        },
        _ = handle => {
            return Ok(());
        },
    };

    terminated_recv.recv().await.unwrap()
}

async fn run_all(listen: &str, reconcile_delay: u64) -> eyre::Result<()> {
    let res = tokio::join!(run_apiserver(listen), run_unitserver(reconcile_delay));
    if res.0.is_err() {
        res.0
    } else if res.1.is_err() {
        res.1
    } else {
        Ok(())
    }
}

pub async fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    match &cli.command {
        Commands::Internal { command } => internal_command(command).await,
        Commands::Systemd { command } => systemd_command(command).await,
        Commands::Start { name } => start_vm(name).await,
        Commands::Stop { name } => stop_vm(name).await,
        Commands::ApiServer { listen } => run_apiserver(listen).await,
        Commands::UnitServer { reconcile_delay } => run_unitserver(*reconcile_delay).await,
        Commands::Serve {
            listen,
            reconcile_delay,
        } => run_all(listen, *reconcile_delay).await,
    }
}
