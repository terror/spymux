use {
  anyhow::{Error, bail},
  app::App,
  command_runner::{CommandRunner, TmuxCommandRunner},
  crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::Stylize,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
  },
  pane::Pane,
  ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
  },
  std::{
    backtrace::BacktraceStatus,
    env,
    io::{self, IsTerminal, Stdout},
    process::{self, Command, Output},
    time::{Duration, Instant},
  },
  terminal_guard::TerminalGuard,
  tmux::Tmux,
  unicode_width::UnicodeWidthChar,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

mod app;
mod command_runner;
mod pane;
mod terminal_guard;
mod tmux;

fn run() -> Result {
  App::new()?.run()
}

fn main() {
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
