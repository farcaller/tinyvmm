use crate::{
    self as tvm,
    database::{entity::Entity, store::Store, virtual_machine::VirtualMachine},
};
use clap::{Parser, Subcommand};
use log::{debug, LevelFilter};
use tokio::{
    signal::{self, unix::SignalKind},
    sync::mpsc,
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    #[clap(long, default_value = "/var/lib/tinyvmm/store.db")]
    store: String,

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
        #[clap(long)]
        api_server: String,

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
        dns_listener: String,

        #[clap(long)]
        api_server: String,
    },
    DnsServer {
        #[clap(long)]
        listen: String,
    },
    Serve {
        #[clap(long)]
        listen: String,

        #[clap(long)]
        listen_dns: String,
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
        #[clap(long)]
        dns_listener: String,
        #[clap(long)]
        dns_server: String,
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

async fn internal_command(cmd: &InternalCommands, store: Store) -> eyre::Result<()> {
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
                    dns_listener,
                    dns_server,
                },
        } => {
            let vms = VirtualMachine::list(&store)?;
            let mut leases = vec![];
            for vm in vms {
                leases.push(tvm::systemd::bridge::Lease {
                    mac: vm.spec.mac,
                    ip: vm.spec.ip,
                });
            }
            tvm::systemd::bridge::create_bridge(name).await?;
            tvm::systemd::bridge::create_bridge_network(
                name,
                dns_zone,
                address,
                dns_listener,
                dns_server,
                leases,
            )
            .await?;
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
        Networkd { command } => networkd_command(command).await?,
    }
    Ok(())
}

async fn systemd_command(cmd: &SystemdCommands, api_server: &str) -> eyre::Result<()> {
    use SystemdCommands::*;

    let client = crate::client::Client::new(api_server.into());

    match cmd {
        BootstrapPre { name } => {
            let tap_name = tvm::ch::get_vm_tap_name(name);

            let vm = client.virtualmachines().get(name).await?;

            tvm::systemd::tap::create_tap(&tap_name, &vm.spec.mac).await?;
            tvm::systemd::tap::create_tap_network(&tap_name, &vm.spec.bridge, &vm.spec.mac).await?;
        }
        BootstrapPost { name } => {
            let vm = client.virtualmachines().get(name).await?;

            tvm::ch::bootstrap::bootstrap_vm(&vm, name).await?
        }
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

async fn run_apiserver(store: Store, listen: &str) -> eyre::Result<()> {
    tvm::apiserver::run_server(listen, store).await?;
    Ok(())
}

async fn run_unitserver(store: Store, dns_listener: &str, api_server: &str) -> eyre::Result<()> {
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);
    let (terminated_send, mut terminated_recv) = mpsc::channel(1);

    let config = tvm::unitserver::Config {
        shutdown_signal: shutdown_recv,
        store,
        dns_listener: dns_listener.into(),
        api_server: api_server.into(),
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

async fn run_dnsserver(store: Store, listen: &str) -> eyre::Result<()> {
    let (shutdown_send, shutdown_recv) = mpsc::channel(1);

    let worker = tvm::dns::run_server(listen.parse()?, store, shutdown_recv);
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

async fn run_all(store: Store, listen: &str, listen_dns: &str) -> eyre::Result<()> {
    let dns_listener = listen_dns.split(':').next().unwrap();
    let res = tokio::join!(
        run_apiserver(store.clone(), listen),
        run_unitserver(store.clone(), dns_listener, listen),
        run_dnsserver(store, listen_dns),
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
        .filter_module("handlebars::render", LevelFilter::Info)
        .filter_module("sled", LevelFilter::Info)
        .filter_module("trust_dns_server", LevelFilter::Info)
        .init();

    match &cli.command {
        Commands::Systemd {
            api_server,
            command,
        } => systemd_command(command, api_server).await,
        Commands::Start { name } => start_vm(name).await,
        Commands::Stop { name } => stop_vm(name).await,

        cmd => {
            let store = Store::new(cli.store.clone())?;

            match cmd {
                Commands::Internal { command } => internal_command(command, store).await,
                Commands::ApiServer { listen } => run_apiserver(store, listen).await,
                Commands::DnsServer { listen } => run_dnsserver(store, listen).await,
                Commands::UnitServer {
                    dns_listener,
                    api_server,
                } => run_unitserver(store, dns_listener, api_server).await,
                Commands::Serve { listen, listen_dns } => run_all(store, listen, listen_dns).await,
                _ => todo!(),
            }
        }
    }
}
