use super::*;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub(crate) struct Pane {
  pub(crate) command: String,
  #[serde(default)]
  pub(crate) content: String,
  pub(crate) id: String,
  pub(crate) index: usize,
  pub(crate) path: String,
  pub(crate) session: String,
  pub(crate) window_index: usize,
}

impl Pane {
  pub(crate) fn descriptor(&self) -> String {
    format!("{}:{}.{}", self.session, self.window_index, self.index)
  }

  pub(crate) fn format<'a>() -> &'a str {
    concat!(
      "{",
      "\"command\":\"#{pane_current_command}\",",
      "\"id\":\"#{pane_id}\",",
      "\"index\":#{pane_index},",
      "\"path\":\"#{pane_current_path}\",",
      "\"session\":\"#{session_name}\",",
      "\"window_index\":#{window_index}",
      "}"
    )
  }

  pub(crate) fn title(&self) -> String {
    let command = self.command.trim();

    if command.is_empty() {
      return self.descriptor();
    }

    format!("{} ({command})", self.descriptor())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn descriptor_displays_session_window_and_index() {
    let pane = Pane {
      index: 1,
      session: "session".into(),
      window_index: 2,
      ..Default::default()
    };

    assert_eq!(pane.descriptor(), "session:2.1");
  }

  #[test]
  fn title_appends_command_when_present() {
    let pane = Pane {
      command: "bash".into(),
      index: 1,
      session: "session".into(),
      window_index: 2,
      ..Default::default()
    };

    assert_eq!(pane.title(), "session:2.1 (bash)");
  }

  #[test]
  fn title_falls_back_to_descriptor_when_command_blank() {
    let pane = Pane {
      index: 1,
      session: "session".into(),
      window_index: 2,
      ..Default::default()
    };

    assert_eq!(pane.title(), "session:2.1");
  }
}
