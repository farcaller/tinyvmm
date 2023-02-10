use std::time::Duration;

use crate::{
    self as tvm,
    database::{entity::Entity, virtual_machine::VirtualMachine},
};
use clap::{Parser, Subcommand};
use log::debug;
use tokio::{
    signal::{self, unix::SignalKind},
    sync::mpsc,
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    #[clap(long, default_value = "/var/lib/tinyvmm")]
    runtime_dir: String,

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
    DnsServer {
        #[clap(long)]
        listen: String,
        #[clap(long)]
        reconcile_delay: u64,
    },
    Serve {
        #[clap(long)]
        listen: String,
        #[clap(long)]
        listen_dns: String,
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
        bridge_name: String,
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

async fn internal_command(cmd: &InternalCommands, runtime_dir: &str) -> eyre::Result<()> {
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
            let vms = VirtualMachine::list(runtime_dir)?;
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
        CreateVMService { name, bridge_name } => {
            tvm::systemd::create_vm_service(
                name,
                bridge_name,
                runtime_dir,
                &std::env::args().next().unwrap(),
            )
            .await?;
        }
        Networkd { command } => networkd_command(command).await?,
    }
    Ok(())
}

async fn systemd_command(cmd: &SystemdCommands, runtime_dir: &str) -> eyre::Result<()> {
    use SystemdCommands::*;
    match cmd {
        BootstrapPre { name } => {
            let tap_name = tvm::ch::get_vm_tap_name(name);

            let vm = VirtualMachine::get(runtime_dir, name)?;

            tvm::systemd::tap::create_tap(&tap_name, &vm.spec.mac).await?;
            tvm::systemd::tap::create_tap_network(&tap_name, &vm.spec.bridge, &vm.spec.mac).await?;
        }
        BootstrapPost { name } => tvm::ch::bootstrap::bootstrap_vm(runtime_dir, name).await?,
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

async fn start_vm(runtime_dir: &str, name: &str) -> eyre::Result<()> {
    tvm::ch::runtime::start_vm(runtime_dir, name).await?;
    Ok(())
}

async fn stop_vm(runtime_dir: &str, name: &str) -> eyre::Result<()> {
    tvm::ch::runtime::shutdown_vm(runtime_dir, name).await?;
    Ok(())
}

async fn run_apiserver(runtime_dir: &str, listen: &str) -> eyre::Result<()> {
    tvm::apiserver::run_server(listen, runtime_dir.into()).await?;
    Ok(())
}

async fn run_unitserver(runtime_dir: &str, reconcile_delay: u64) -> eyre::Result<()> {
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);
    let (terminated_send, mut terminated_recv) = mpsc::channel(1);

    let config = tvm::unitserver::Config {
        reconcile_delay: Duration::from_secs(reconcile_delay),
        shutdown_signal: shutdown_recv,
        runtime_dir: runtime_dir.into(),
    };

    let worker = tvm::unitserver::main(config, terminated_send);
    let handle = tokio::spawn(worker);
    let sig_int = signal::ctrl_c();
    let mut sig_quit = signal::unix::signal(SignalKind::quit())?;
    let mut sig_term = signal::unix::signal(SignalKind::terminate())?;

    tokio::select! {
        _ = sig_int => {
            debug!("SIGINT received; starting forced shutdown");
            shutdown_send.send(()).await?;
        },
        _ = sig_quit.recv() => {
            debug!("SIGQUIT received; starting forced shutdown");
            shutdown_send.send(()).await?;
        },
        _ = sig_term.recv() => {
            debug!("SIGTERM received; starting graceful shutdown");
            shutdown_send.send(()).await?;
        },
        _ = handle => {
            return Ok(());
        },
    };

    terminated_recv.recv().await.unwrap()
}

async fn run_dnsserver(runtime_dir: &str, reconcile_delay: u64, listen: &str) -> eyre::Result<()> {
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);

    let worker = tvm::dns::run_server(
        listen.parse()?,
        runtime_dir.into(),
        Duration::from_secs(reconcile_delay),
        shutdown_recv,
    );
    let handle = tokio::spawn(worker);
    let sig_int = signal::ctrl_c();
    let mut sig_quit = signal::unix::signal(SignalKind::quit())?;
    let mut sig_term = signal::unix::signal(SignalKind::terminate())?;

    tokio::spawn(async move {
        tokio::select! {
            _ = sig_int => {
                debug!("SIGINT received; starting forced shutdown");
                shutdown_send.send(()).await?;
            },
            _ = sig_quit.recv() => {
                debug!("SIGQUIT received; starting forced shutdown");
                shutdown_send.send(()).await?;
            },
            _ = sig_term.recv() => {
                debug!("SIGTERM received; starting graceful shutdown");
                shutdown_send.send(()).await?;
            },
        };
        Ok::<(), eyre::Report>(())
    });

    handle.await.unwrap()
}

async fn run_all(
    runtime_dir: &str,
    listen: &str,
    listen_dns: &str,
    reconcile_delay: u64,
) -> eyre::Result<()> {
    let res = tokio::join!(
        run_apiserver(runtime_dir, listen),
        run_unitserver(runtime_dir, reconcile_delay),
        run_dnsserver(runtime_dir, reconcile_delay, listen_dns),
    );
    if res.0.is_err() {
        res.0
    } else if res.1.is_err() {
        res.1
    } else if (res.2).is_err() {
        res.2
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
        Commands::Internal { command } => internal_command(command, &cli.runtime_dir).await,
        Commands::Systemd { command } => systemd_command(command, &cli.runtime_dir).await,
        Commands::Start { name } => start_vm(name, &cli.runtime_dir).await,
        Commands::Stop { name } => stop_vm(name, &cli.runtime_dir).await,
        Commands::ApiServer { listen } => run_apiserver(&cli.runtime_dir, listen).await,
        Commands::DnsServer {
            reconcile_delay,
            listen,
        } => run_dnsserver(&cli.runtime_dir, *reconcile_delay, listen).await,
        Commands::UnitServer { reconcile_delay } => {
            run_unitserver(&cli.runtime_dir, *reconcile_delay).await
        }
        Commands::Serve {
            listen,
            listen_dns,
            reconcile_delay,
        } => run_all(&cli.runtime_dir, listen, listen_dns, *reconcile_delay).await,
    }
}
