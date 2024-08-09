use std::process::ExitCode;

use anyhow::Context;
use pico_args::Arguments;

mod run_wasm;
mod util;

const HELP: &str = "\
Usage: xtask <COMMAND>

Commands:
  run-wasm
    Build and run wasm

    --release   Build in release mode
    --no-serve  Just build the generated files, don't serve them

Options:
  -h, --help  Print help
";

/// Helper macro for printing the help message, then bailing with an error message.
#[macro_export]
macro_rules! bad_arguments {
    ($($arg:tt)*) => {{
        eprintln!("{}", crate::HELP);
        anyhow::bail!($($arg)*)
    }};
}

fn main() -> anyhow::Result<ExitCode> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .format_indent(Some(0))
        .init();

    let mut args = Arguments::from_env();

    if args.contains("--help") {
        eprint!("{HELP}");
        return Ok(ExitCode::FAILURE);
    }

    let subcommand = args
        .subcommand()
        .context("Expected subcommand to be UTF-8")?;

    // -- Shell Creation --

    let shell = xshell::Shell::new().context("Couldn't create xshell shell")?;
    shell.change_dir(String::from(env!("CARGO_MANIFEST_DIR")) + "/..");

    match subcommand.as_deref() {
        Some("run-wasm") => run_wasm::run_wasm(shell, args)?,
        Some(subcommand) => {
            bad_arguments!("Unknown subcommand: {}", subcommand)
        }
        None => {
            bad_arguments!("Expected subcommand")
        }
    }

    Ok(ExitCode::SUCCESS)
}
