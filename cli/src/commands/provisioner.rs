use crate::helper::tokio_runtime;
use crate::CLIError;
use bluetooth_mesh::replay;
use bluetooth_mesh::stack::bearer::IncomingMessage;
use bluetooth_mesh::stack::full::FullStack;
use bluetooth_mesh::stack::StackInternals;
use btle::le::report::ReportInfo;
use futures_util::StreamExt;

pub fn sub_command() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name("provisioner")
        .about("Provisioner Role for adding Nodes to a network")
        .subcommand(
            clap::SubCommand::with_name("run")
                .about("join real Bluetooth Mesh network as a provisioner."),
        )
}
pub fn provisioner_matches(
    logger: &slog::Logger,
    device_state_path: &str,
    matches: &clap::ArgMatches,
) -> Result<(), CLIError> {
    match matches.subcommand() {
        ("run", Some(_matches)) => tokio_runtime().block_on(provision(logger, device_state_path)),
        ("", None) => Err(CLIError::Clap(clap::Error::with_description(
            "missing subcommand",
            clap::ErrorKind::ArgumentNotFound,
        ))),
        _ => unreachable!("unhandled provisioner subcommand"),
    }
}

pub async fn provision(_logger: &slog::Logger, device_state_path: &str) -> Result<(), CLIError> {
    let dsm = crate::helper::load_device_state(device_state_path)?;
    let (adapter, adapter_source) = crate::helper::hci_adapter();
    println!("using hci adapter from '{}'", adapter_source);
    futures_util::pin_mut!(adapter);
    let adapter = btle::hci::adapters::Adapter::new(adapter);
    let mut le = adapter.le();
    async move {
        let incoming = le.advertisement_stream::<Box<[ReportInfo]>>().await?;
        futures_util::pin_mut!(incoming);
        let internals = StackInternals::new(dsm);
        let cache = replay::Cache::new();
        let mut stack = FullStack::new(internals, cache, 5);
        while let Some(report_info) = incoming.next().await {
            if let Some(new_msg) = IncomingMessage::from_report_info(report_info?) {
                dbg!(&new_msg);
                match new_msg {
                    IncomingMessage::Network(n) => {
                        if stack.incoming_bearer.send(n).await.is_err() {
                            break;
                        }
                    }
                    IncomingMessage::Beacon(b) => todo!("handle beacons {:?}", b),
                    IncomingMessage::PBAdv(p) => todo!("handle pb_adv {:?}", p),
                }
            }
        }
        Result::<(), Box<dyn btle::error::Error>>::Ok(())
    }
    .await
    .map_err(|e| CLIError::OtherMessage(format!("stack error: {:?}", e)))?;
    println!("provisioner done");
    Ok(())
}
