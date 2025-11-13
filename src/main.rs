use {
  anyhow::{Error, bail},
  command_runner::{CommandRunner, TmuxCommandRunner},
  crossterm::{
    execute,
    style::Stylize,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
  },
  pane::Pane,
  ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem},
  },
  std::{
    backtrace::BacktraceStatus,
    io::{self, IsTerminal, Stdout},
    process::{self, Command, Output},
  },
  terminal_guard::TerminalGuard,
  tmux::Tmux,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

mod command_runner;
mod pane;
mod terminal_guard;
mod tmux;

#[derive(Debug)]
struct App {
  terminal: TerminalGuard,
  tmux: Tmux,
}

impl App {
  fn draw(&mut self, frame: &mut Frame) {
    let items: Vec<ListItem> = if self.tmux.panes.is_empty() {
      vec![ListItem::new("No tmux panes detected")]
    } else {
      self
        .tmux
        .panes
        .iter()
        .map(|pane| {
          let preview = pane
            .content
            .lines()
            .find(|line| !line.trim().is_empty())
            .map_or_else(
              || "(empty pane)".to_string(),
              |line| line.trim().to_string(),
            );

          ListItem::new(format!("{} | {preview}", pane.id))
        })
        .collect()
    };

    let list = List::new(items)
      .block(Block::default().title("tmux panes").borders(Borders::ALL));

    frame.render_widget(list, frame.area());
  }

  fn new() -> Result<Self> {
    Ok(Self {
      terminal: TerminalGuard::new()?,
      tmux: Tmux::capture()?,
    })
  }

  fn run(self) -> Result {
    Ok(())
  }
}

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
