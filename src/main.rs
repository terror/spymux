use {
  anyhow::{Error, bail},
  command_runner::{CommandRunner, TmuxCommandRunner},
  crossterm::{
    execute,
    style::Stylize,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
  },
  pane::Pane,
  ratatui::{Terminal, backend::CrosstermBackend},
  std::{
    backtrace::BacktraceStatus,
    io::{self, IsTerminal, Stdout},
    process::{self, Command, Output},
  },
  terminal_guard::TerminalGuard,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

mod command_runner;
mod pane;
mod terminal_guard;
mod tmux;

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
