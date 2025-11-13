use super::*;

pub(crate) trait CommandRunner {
  fn run(&self, args: &[&str]) -> Result<Output>;
}

pub(crate) struct TmuxCommandRunner;

impl CommandRunner for TmuxCommandRunner {
  fn run(&self, args: &[&str]) -> Result<Output> {
    Ok(Command::new("tmux").args(args).output()?)
  }
}
