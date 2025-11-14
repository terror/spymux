use super::*;

#[derive(Debug, Clone, Parser)]
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
  #[clap(flatten)]
  pub(crate) options: Options,
  #[clap(subcommand)]
  pub(crate) subcommand: Option<Subcommand>,
}

impl Arguments {
  pub(crate) fn run(self) -> Result {
    match self.subcommand {
      Some(subcommand) => subcommand.run(),
      None => App::new(Config {
        color_output: !self.options.no_colors,
      })?
      .run(),
    }
  }
}
