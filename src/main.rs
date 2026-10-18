use clap::Parser;
use marketfeed::MarketfeedCommand;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "marketfeed",
    about = "Typed Rust client library and CLI for financial market and currency data sources",
    version
)]
struct Cli {
    #[command(flatten)]
    cmd: MarketfeedCommand,
}

impl Cli {
    fn run(self) -> ExitCode {
        match self.cmd.run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::from(2)
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    cli.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[googletest::test]
    fn run_cli_default_returns_success() {
        let cli = Cli {
            cmd: MarketfeedCommand::default(),
        };
        assert_that!(cli.run(), eq(ExitCode::SUCCESS));
    }
}
