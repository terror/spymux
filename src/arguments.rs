use {super::*, clap::Subcommand};

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
  #[command(subcommand)]
  command: Option<Command>,
  #[arg(short, long, help = "Disable colored output")]
  no_colors: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
  #[command(about = "Resume a running spymux instance via fzf")]
  Resume,
}

impl Arguments {
  pub(crate) fn color_output(&self) -> bool {
    !self.no_colors
  }

  pub(crate) fn run(&self) -> Result {
    match self.command {
      Some(Command::Resume) => resume::run(),
      None => App::new(Config {
        color_output: self.color_output(),
      })?
      .run(),
    }
  }
}
