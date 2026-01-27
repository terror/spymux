use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Config {
  pub(crate) color_output: bool,
  pub(crate) command_filter: Vec<String>,
  pub(crate) refresh_rate: Duration,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      color_output: true,
      command_filter: Vec::new(),
      refresh_rate: Duration::from_millis(500),
    }
  }
}
