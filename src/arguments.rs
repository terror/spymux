use super::*;

#[derive(Debug, Parser)]
#[clap(
  about,
  author,
  version,
  help_template = "\
{before-help}{name} {version}

{about}

\x1b[1;4mUsage\x1b[0m: {usage}

{all-args}{after-help}
"
)]
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
