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
    if let Some(subcommand) = self.subcommand {
      subcommand.run()
    } else {
      let refresh_rate = self.options.refresh_rate.map_or_else(
        || Config::default().refresh_rate,
        |rate| Duration::from_millis(rate.get()),
      );

      App::new(Config {
        color_output: !self.options.no_colors,
        refresh_rate,
      })?
      .run()
    }
  }
}
