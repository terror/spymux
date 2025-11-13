#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pane {
  pub(crate) content: String,
  pub(crate) id: String,
  pub(crate) pane_index: usize,
  pub(crate) session: String,
  pub(crate) tmux_pane_id: String,
  pub(crate) window: usize,
}
