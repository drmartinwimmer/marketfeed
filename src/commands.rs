use clap::Args;

/// Error type for `marketfeed` command execution.
#[derive(Debug, thiserror::Error)]
pub enum MarketfeedError {
    /// I/O error during command execution.
    #[error("I/O error during marketfeed execution: {0}")]
    Io(#[from] std::io::Error),
}

/// Command-line arguments for `marketfeed`.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketfeedCommand {
    /// Optional provider filter or query target
    #[arg(long)]
    provider: Option<String>,
}

impl MarketfeedCommand {
    /// Creates a new `MarketfeedCommand`.
    pub fn new(provider: Option<String>) -> Self {
        Self { provider }
    }

    /// Executes the command.
    pub fn run(self) -> Result<(), MarketfeedError> {
        let _ = self.provider;
        Ok(())
    }
}
