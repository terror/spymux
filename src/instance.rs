use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Instance {
  pub(crate) current_path: String,
  pub(crate) pane: Pane,
}
