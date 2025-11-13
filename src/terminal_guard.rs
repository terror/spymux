use super::*;

pub(crate) struct TerminalGuard {
  terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      terminal: initialize_terminal()?,
    })
  }

  pub(crate) fn restore(&mut self) -> Result {
    terminal::disable_raw_mode()?;
    execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
    self.terminal.show_cursor()?;
    Ok(())
  }

  #[allow(dead_code)]
  pub(crate) fn terminal_mut(
    &mut self,
  ) -> &mut Terminal<CrosstermBackend<Stdout>> {
    &mut self.terminal
  }
}

impl Drop for TerminalGuard {
  fn drop(&mut self) {
    if let Err(error) = self.restore() {
      eprintln!("failed to restore terminal: {error}");
    }
  }
}
