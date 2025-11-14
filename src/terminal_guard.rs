use super::*;

#[derive(Debug)]
pub(crate) struct TerminalGuard {
  pub(crate) terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
  fn initialize() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
  }

  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      terminal: Self::initialize()?,
    })
  }

  pub(crate) fn restore(&mut self) -> Result {
    terminal::disable_raw_mode()?;

    execute!(
      self.terminal.backend_mut(),
      LeaveAlternateScreen,
      DisableMouseCapture
    )?;

    self.terminal.show_cursor()?;

    Ok(())
  }

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
