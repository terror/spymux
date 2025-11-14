#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Action {
  FocusPane(String),
  Quit,
}
