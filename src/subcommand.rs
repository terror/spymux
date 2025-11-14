use super::*;

mod resume;

#[derive(Debug, Clone, Parser)]
pub(crate) enum Subcommand {
  Resume,
}

impl Subcommand {
  pub(crate) fn run(self) -> Result {
    match self {
      Self::Resume => resume::run(),
    }
  }
}
