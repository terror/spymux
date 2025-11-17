use super::*;

#[derive(Debug, Clone, Parser)]
pub(crate) struct Options {
  #[clap(short, long, help = "Disable colored output")]
  pub(crate) no_colors: bool,
  #[clap(
    long = "refresh-rate",
    value_name = "MILLISECONDS",
    value_parser = clap::value_parser!(NonZeroU64),
    help = "Refresh interval in milliseconds (default: 500)"
  )]
  pub(crate) refresh_rate: Option<NonZeroU64>,
}
