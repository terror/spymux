use {super::*, unicode_width::UnicodeWidthChar};

#[derive(Debug)]
pub(crate) struct App {
  current_pane_id: Option<String>,
  last_refresh: Instant,
  terminal: TerminalGuard,
  tmux: Tmux,
}

impl App {
  const REFRESH_INTERVAL: Duration = Duration::from_millis(500);

  fn capture_tmux(current_pane_id: Option<&str>) -> Result<Tmux> {
    let mut tmux = Tmux::capture()?;

    if let Some(pane_id) = current_pane_id {
      tmux.exclude_pane_id(pane_id);
    }

    Ok(tmux)
  }

  fn clip_to_bottom(
    content: &str,
    max_lines: usize,
    max_columns: usize,
  ) -> String {
    if max_lines == 0 || max_columns == 0 || content.is_empty() {
      return String::new();
    }

    let bytes = content.as_bytes();
    let mut render_end = bytes.len();

    while render_end > 0 && bytes[render_end - 1] == b'\n' {
      render_end -= 1;
    }

    if render_end == 0 {
      return content.to_string();
    }

    let mut row_starts = vec![0usize];
    let mut total_rows = 1usize;
    let mut current_width = 0usize;

    for (idx, ch) in content.char_indices() {
      if idx >= render_end {
        break;
      }

      if ch == '\n' {
        current_width = 0;
        total_rows += 1;
        row_starts.push(idx + ch.len_utf8());
        continue;
      }

      let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

      if ch_width == 0 {
        continue;
      }

      if current_width.saturating_add(ch_width) > max_columns {
        current_width = 0;
        total_rows += 1;
        row_starts.push(idx);
      }

      current_width += ch_width;
    }

    if total_rows <= max_lines {
      return content.to_string();
    }

    let rows_to_skip = total_rows - max_lines;
    let start_index = *row_starts.get(rows_to_skip).unwrap_or(&render_end);

    content[start_index..].to_string()
  }

  pub(crate) fn new() -> Result<Self> {
    let terminal = TerminalGuard::new()?;

    let current_pane_id = env::var("TMUX_PANE").ok();

    Ok(Self {
      terminal,
      tmux: Self::capture_tmux(current_pane_id.as_deref())?,
      current_pane_id,
      last_refresh: Instant::now(),
    })
  }

  fn refresh_tmux(&mut self) -> Result {
    self.tmux = Self::capture_tmux(self.current_pane_id.as_deref())?;
    self.last_refresh = Instant::now();
    Ok(())
  }

  pub(crate) fn run(mut self) -> Result {
    loop {
      if self.last_refresh.elapsed() >= Self::REFRESH_INTERVAL {
        self.refresh_tmux()?;
      }

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
          let inner_height = pane_area.height.saturating_sub(2);
          let inner_width = pane_area.width.saturating_sub(2);

          let visible_lines = usize::from(inner_height);
          let visible_columns = usize::from(inner_width);

          let clipped_content =
            Self::clip_to_bottom(&pane.content, visible_lines, visible_columns);

          let widget = Paragraph::new(clipped_content)
            .wrap(Wrap { trim: false })
            .block(
              Block::default()
                .title(pane.id.clone())
                .borders(Borders::ALL),
            );

          frame.render_widget(widget, pane_area);
        }
      })?;

      let timeout = Self::REFRESH_INTERVAL
        .checked_sub(self.last_refresh.elapsed())
        .unwrap_or(Duration::from_millis(0));

      if event::poll(timeout)?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
      {
        break;
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::App;

  #[test]
  fn clip_to_bottom_returns_all_when_shorter() {
    let content = "line1\nline2";
    assert_eq!(App::clip_to_bottom(content, 5, 80), content);
  }

  #[test]
  fn clip_to_bottom_limits_to_requested_lines() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2\nline3\nline4", 2, 80),
      "line3\nline4"
    );
  }

  #[test]
  fn clip_to_bottom_handles_trailing_newlines() {
    assert_eq!(App::clip_to_bottom("line1\nline2\n", 1, 80), "line2\n");
  }

  #[test]
  fn clip_to_bottom_with_zero_lines_returns_empty() {
    assert_eq!(App::clip_to_bottom("line1\nline2", 0, 80), "");
  }

  #[test]
  fn clip_to_bottom_truncates_wrapped_lines() {
    let content = "1234567890abcdefghij";

    assert_eq!(App::clip_to_bottom(content, 2, 5), "abcdefghij");
  }

  #[test]
  fn clip_to_bottom_with_zero_columns_returns_empty() {
    assert_eq!(App::clip_to_bottom("line1\nline2", 5, 0), "");
  }
}
