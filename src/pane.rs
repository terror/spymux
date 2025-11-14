use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Pane {
  pub(crate) command: String,
  #[serde(default)]
  pub(crate) content: String,
  pub(crate) pane_index: usize,
  pub(crate) path: String,
  pub(crate) session: String,
  #[serde(rename = "pane_id")]
  pub(crate) tmux_pane_id: String,
  #[serde(rename = "window_index")]
  pub(crate) window: usize,
}

impl Pane {
  pub(crate) fn descriptor(&self) -> String {
    format!("{}:{}.{}", self.session, self.window, self.pane_index)
  }

  pub(crate) fn format<'a>() -> &'a str {
    concat!(
      "{",
      "\"command\":\"#{pane_current_command}\",",
      "\"pane_id\":\"#{pane_id}\",",
      "\"pane_index\":#{pane_index},",
      "\"path\":\"#{pane_current_path}\",",
      "\"session\":\"#{session_name}\",",
      "\"window_index\":#{window_index},",
      "}"
    )
  }
}
