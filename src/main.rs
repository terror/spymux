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
    Terminal,
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem},
  },
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
  let mut terminal = TerminalGuard::new()?;
  let tmux = tmux::Tmux::capture()?;
  render_tmux_panes(terminal.terminal_mut(), tmux.panes())
}

fn render_tmux_panes(
  terminal: &mut Terminal<CrosstermBackend<Stdout>>,
  panes: &[Pane],
) -> Result {
  terminal.draw(|frame| {
    let items: Vec<ListItem> = if panes.is_empty() {
      vec![ListItem::new("No tmux panes detected")]
    } else {
      panes
        .iter()
        .map(|pane| {
          let preview = pane
            .content
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .unwrap_or_else(|| "(empty pane)".to_string());

          ListItem::new(format!("{} | {preview}", pane.id))
        })
        .collect()
    };

    let list = List::new(items)
      .block(Block::default().title("tmux panes").borders(Borders::ALL));

    frame.render_widget(list, frame.area());
  })?;

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
