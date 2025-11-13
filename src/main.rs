use {
  crossterm::{
    execute,
    style::Stylize,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
  },
  ratatui::{Terminal, backend::CrosstermBackend},
  std::{
    backtrace::BacktraceStatus,
    io::{self, IsTerminal, Stdout},
    process,
  },
};

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

struct TerminalGuard {
  terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
  fn new() -> Result<Self> {
    Ok(Self {
      terminal: initialize_terminal()?,
    })
  }

  fn restore(&mut self) -> Result {
    terminal::disable_raw_mode()?;
    execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
    self.terminal.show_cursor()?;
    Ok(())
  }

  #[allow(dead_code)]
  fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
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

fn initialize_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
  terminal::enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen)?;
  Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn run() -> Result {
  let _terminal = TerminalGuard::new()?;
  Ok(())
}

#[tokio::main]
async fn main() {
  if let Err(error) = run() {
    let use_color = io::stderr().is_terminal();

    if use_color {
      eprintln!("{} {error}", "error:".bold().red());
    } else {
      eprintln!("error: {error}");
    }

    for (i, error) in error.chain().skip(1).enumerate() {
      if i == 0 {
        eprintln!();

        if use_color {
          eprintln!("{}", "because:".bold().red());
        } else {
          eprintln!("because:");
        }
      }

      if use_color {
        eprintln!("{} {error}", "-".bold().red());
      } else {
        eprintln!("- {error}");
      }
    }

    let backtrace = error.backtrace();

    if backtrace.status() == BacktraceStatus::Captured {
      if use_color {
        eprintln!("{}", "backtrace:".bold().red());
      } else {
        eprintln!("backtrace:");
      }

      eprintln!("{backtrace}");
    }

    process::exit(1);
  }
}
