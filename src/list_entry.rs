use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ListEntry {
  pub(crate) command: String,
  pub(crate) pane_id: String,
  pub(crate) pane_index: usize,
  pub(crate) path: String,
  pub(crate) session: String,
  pub(crate) window_index: usize,
}

impl ListEntry {
  pub(crate) fn descriptor(&self) -> String {
    format!("{}:{}.{}", self.session, self.window_index, self.pane_index)
  }

  pub(crate) fn format<'a>() -> &'a str {
    concat!(
      "{",
      "\"command\":\"#{pane_current_command}\",",
      "\"pane_id\":\"#{pane_id}\",",
      "\"pane_index\":#{pane_index},",
      "\"path\":\"#{pane_current_path}\"",
      "\"session\":\"#{session_name}\",",
      "\"window_index\":#{window_index},",
      "}"
    )
  }
}
