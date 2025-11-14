use super::*;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Action {
  FocusPane(Pane),
  Quit,
}
