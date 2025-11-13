#[derive(Clone, Copy, Debug)]
pub(crate) struct Config {
  pub(crate) color_output: bool,
}

impl Default for Config {
  fn default() -> Self {
    Self { color_output: true }
  }
}
