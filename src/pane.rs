use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
    let descriptor = self.descriptor();

    let command = self.command.trim();

    if command.is_empty() {
      descriptor
    } else {
      format!("{descriptor} ({command})")
    }
  }
}
