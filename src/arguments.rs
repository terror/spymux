use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Arguments {
  #[arg(short, long, help = "Disable colored output")]
  no_colors: bool,
}

impl Arguments {
  pub(crate) fn color_output(&self) -> bool {
    !self.no_colors
  }

  pub(crate) fn run(&self) -> Result {
    App::new(Config {
      color_output: self.color_output(),
    })?
    .run()
  }
}
