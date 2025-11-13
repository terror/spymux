use super::*;

#[derive(Debug)]
pub(crate) struct App {
  terminal: TerminalGuard,
  tmux: Tmux,
}

impl App {
  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      terminal: TerminalGuard::new()?,
      tmux: Tmux::capture()?,
    })
  }

  pub(crate) fn run(mut self) -> Result {
    loop {
      let terminal = self.terminal.terminal_mut();

      terminal.draw(|frame| {
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
      })?;

      if let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
      {
        break;
      }
    }

    Ok(())
  }
}
