use {
  ansi_to_tui::IntoText,
  anyhow::{Error, bail},
  app::App,
  arguments::Arguments,
  clap::Parser,
  command_runner::{CommandRunner, TmuxCommandRunner},
  config::Config,
  crossterm::{
    event::{
      self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
      KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    style::Stylize,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
  },
  pane::Pane,
  ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
  },
  row_cursor::RowCursor,
  std::{
    backtrace::BacktraceStatus,
    borrow::Cow,
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
mod arguments;
mod command_runner;
mod config;
mod pane;
mod row_cursor;
mod terminal_guard;
mod tmux;

fn main() {
  let arguments = Arguments::parse();

  if let Err(error) = arguments.run() {
    let use_color = io::stderr().is_terminal() && arguments.color_output();

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
