use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Config {
  pub(crate) color_output: bool,
  pub(crate) command_filter: Vec<String>,
  pub(crate) refresh_rate: Duration,
}

impl Config {
  pub(crate) fn style(&self, color: Color) -> Style {
    if self.color_output {
      Style::default().fg(color)
    } else {
      Style::default()
    }
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn style_respects_color_output() {
    let mut config = Config::default();

    assert_eq!(config.style(Color::Cyan), Style::default().fg(Color::Cyan));

    config.color_output = false;

    assert_eq!(config.style(Color::Cyan), Style::default());
  }
}
