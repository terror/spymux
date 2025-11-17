use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Config {
  pub(crate) color_output: bool,
  pub(crate) refresh_rate: Duration,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      color_output: true,
      refresh_rate: Duration::from_millis(500),
    }
  }
}
