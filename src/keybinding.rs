#[derive(Debug, Clone, Copy)]
pub(crate) struct Keybinding {
  pub(crate) description: &'static str,
  pub(crate) keys: &'static str,
}
