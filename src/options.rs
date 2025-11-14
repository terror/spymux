use super::*;

#[derive(Debug, Clone, Parser)]
pub(crate) struct Options {
  #[clap(short, long, help = "Disable colored output")]
  pub(crate) no_colors: bool,
}
