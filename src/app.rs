use super::*;

#[derive(Debug)]
pub(crate) struct App {
  terminal: TerminalGuard,
  tmux: Tmux,
}

impl App {
  pub(crate) fn new() -> Result<Self> {
    let terminal = TerminalGuard::new()?;

    let mut tmux = Tmux::capture()?;

    if let Ok(current_pane) = std::env::var("TMUX_PANE") {
      tmux.exclude_pane_id(&current_pane);
    }

    Ok(Self { terminal, tmux })
  }

  pub(crate) fn run(mut self) -> Result {
    loop {
      let terminal = self.terminal.terminal_mut();

      terminal.draw(|frame| {
        let area = frame.area();

        if self.tmux.panes.is_empty() {
          let widget = Paragraph::new("No tmux panes detected")
            .block(Block::default().title("tmux panes").borders(Borders::ALL));

          frame.render_widget(widget, area);

          return;
        }

        let pane_count = self.tmux.panes.len();

        let mut columns: usize = 1;

        while columns.saturating_mul(columns) < pane_count {
          columns += 1;
        }

        let rows = pane_count.div_ceil(columns);

        let (rows_u32, columns_u32) = (
          u32::try_from(rows).unwrap_or(u32::MAX),
          u32::try_from(columns).unwrap_or(u32::MAX),
        );

        let row_constraints = vec![Constraint::Ratio(1, rows_u32); rows];

        let row_chunks = Layout::default()
          .direction(Direction::Vertical)
          .constraints(row_constraints)
          .split(area);

        let mut pane_areas = Vec::with_capacity(pane_count);

        'outer: for row_chunk in row_chunks.iter().copied() {
          let column_constraints =
            vec![Constraint::Ratio(1, columns_u32); columns];

          let column_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(column_constraints)
            .split(row_chunk);

          for column_chunk in column_chunks.iter().copied() {
            pane_areas.push(column_chunk);

            if pane_areas.len() == pane_count {
              break 'outer;
            }
          }
        }

        for (pane, pane_area) in self.tmux.panes.iter().zip(pane_areas) {
          let widget = Paragraph::new(pane.content.clone())
            .wrap(Wrap { trim: false })
            .block(
              Block::default()
                .title(pane.id.clone())
                .borders(Borders::ALL),
            );

          frame.render_widget(widget, pane_area);
        }
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
