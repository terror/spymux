use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Arguments {
  #[arg(short, long, help = "Disable colored output")]
  no_colors: bool,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    App::new()?.run()
  }
}
