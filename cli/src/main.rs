use bluetooth_mesh::mesh::ElementCount;
use slog::Drain;
#[macro_use]
extern crate slog;

use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::error::Error;
pub mod commands;
pub mod helper;
pub enum CLIError {
    IOError(String, std::io::Error),
    Clap(clap::Error),
    SerdeJSON(serde_json::Error)
}
fn main() {
    let app = clap::App::new("Bluetooth Mesh CLI")
        .version(clap::crate_version!())
        .author("Andrew Gilbrough <andrew@gilbrough.com>")
        .about("Bluetooth Mesh Command Line Interface tool to interact with the Mesh")
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .max_values(5)
                .help("Set the amount of logging from level 0 up to level 5"),
        )
        .arg(
            clap::Arg::with_name("device_state")
                .short("s")
                .long("device_state")
                .value_name("FILE")
                .help("Specifies device state .json file"),
        )
        .subcommand(commands::generate::sub_command())
        .subcommand(commands::provisioner::sub_command())
        .subcommand(commands::crypto::sub_command());
    let matches = app.get_matches();

    let log_level = slog::Level::from_usize(
        1 + usize::try_from(matches.occurrences_of("verbose"))
            .expect("verbose usize overflow (how??)"),
    )
    .expect("verbose limit set too low");
    let drain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let root = slog::Logger::root(slog_term::FullFormat::new(drain).build().fuse(), slog::o!());
    /*
    let root = slog::LevelFilter::new(
        slog::Logger::root(slog_term::FullFormat::new(drain).build().fuse(), slog::o!()),
        log_level,
    );
    */
    trace!(root, "main");
    let sub_cmd = matches.subcommand().0;
    let get_device_state_path = || -> &str {
        match matches.value_of("device_state") {
            Some(path) => path,
            None => clap::Error::with_description("missing 'device_state.json` path", clap::ErrorKind::ArgumentNotFound).exit()
        }
    };
    debug!(root, "arg_match"; "sub_command" => sub_cmd);
    if let Err(e) = (|| -> Result<(), CLIError> {
        match matches.subcommand() {
            ("", None) => error!(root, "no command given"),
            ("generate", Some(gen_matches)) => commands::generate::generate_matches(&root, get_device_state_path(), gen_matches)?,
            ("crypto", Some(crypto_matches)) => commands::crypto::crypto_matches(&root, get_device_state_path(), crypto_matches)?,
            ("provisioner", Some(prov_matches)) => commands::provisioner::provisioner_matches(&root, get_device_state_path(), prov_matches)?,
            _ => unreachable!("unhandled sub_command"),
        }
        debug!(root, "matches_done");
        Ok(())
    })() {
        use std::io::Write;
        let mut stderr = std::io::stderr();
        match e {
            CLIError::IOError(path, error) => writeln!(&mut stderr, "io error {} with path '{}'", error.description(), path).ok(),
            CLIError::Clap(error) => writeln!(&mut stderr, "{}", &error.message).ok(),
            CLIError::SerdeJSON(error) => writeln!(&mut stderr, "json error {}", error).ok(),
        };
        std::process::exit(0);
    }
    ()
}