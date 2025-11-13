use {
  super::*,
  ansi_to_tui::IntoText,
  ratatui::text::{Line, Text},
  std::borrow::Cow,
  unicode_width::UnicodeWidthChar,
};

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
  ) -> Text<'static> {
    if max_lines == 0 || max_columns == 0 || content.is_empty() {
      return Text::default();
    }

    let parsed_text = content
      .into_text()
      .unwrap_or_else(|_| Text::raw(content.to_string()));

    let renderable_lines = Self::renderable_line_count(&parsed_text);

    if renderable_lines == 0 {
      return parsed_text;
    }

    let row_starts =
      Self::collect_row_starts(&parsed_text, max_columns, renderable_lines);

    if row_starts.len() <= max_lines {
      return parsed_text;
    }

    let rows_to_skip = row_starts.len().saturating_sub(max_lines);

    let start_cursor = row_starts
      .get(rows_to_skip)
      .copied()
      .unwrap_or_else(RowCursor::default);

    Self::slice_text_from(&parsed_text, start_cursor)
  }

  fn collect_row_starts(
    text: &Text<'static>,
    max_columns: usize,
    line_limit: usize,
  ) -> Vec<RowCursor> {
    let mut starts = Vec::new();

    if line_limit == 0 {
      return starts;
    }

    starts.push(RowCursor {
      byte_index: 0,
      line_index: 0,
      span_index: 0,
    });

    let mut current_width = 0usize;

    for line_index in 0..line_limit {
      let line = &text.lines[line_index];

      if line.spans.is_empty() {
        current_width = 0;

        if line_index + 1 < line_limit {
          starts.push(RowCursor {
            byte_index: 0,
            line_index: line_index + 1,
            span_index: 0,
          });
        }

        continue;
      }

      for (span_index, span) in line.spans.iter().enumerate() {
        let mut byte_index = 0usize;

        let content = span.content.as_ref();

        for ch in content.chars() {
          let ch_len = ch.len_utf8();

          let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

          if ch_width > 0
            && current_width > 0
            && current_width.saturating_add(ch_width) > max_columns
          {
            starts.push(RowCursor {
              byte_index,
              line_index,
              span_index,
            });

            current_width = 0;
          }

          byte_index += ch_len;

          current_width = current_width.saturating_add(ch_width);
        }
      }

      current_width = 0;

      if line_index + 1 < line_limit {
        starts.push(RowCursor {
          byte_index: 0,
          line_index: line_index + 1,
          span_index: 0,
        });
      }
    }

    starts
  }

  fn line_is_empty(line: &Line<'_>) -> bool {
    if line.spans.is_empty() {
      return true;
    }

    line.spans.iter().all(|span| span.content.is_empty())
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

  fn renderable_line_count(text: &Text<'static>) -> usize {
    let mut end = text.lines.len();

    while end > 0 && Self::line_is_empty(&text.lines[end - 1]) {
      end -= 1;
    }

    end
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

  fn slice_text_from(text: &Text<'static>, cursor: RowCursor) -> Text<'static> {
    let mut lines = Vec::new();

    for (line_index, line) in
      text.lines.iter().enumerate().skip(cursor.line_index)
    {
      let mut new_line = Line {
        style: line.style,
        alignment: line.alignment,
        spans: Vec::new(),
      };

      if line.spans.is_empty() {
        lines.push(new_line);
        continue;
      }

      let start_span = if line_index == cursor.line_index {
        cursor.span_index.min(line.spans.len())
      } else {
        0
      };

      for (span_index, span) in line.spans.iter().enumerate().skip(start_span) {
        let mut new_span = span.clone();

        if line_index == cursor.line_index && span_index == cursor.span_index {
          let source = span.content.as_ref();

          if cursor.byte_index >= source.len() {
            continue;
          }

          new_span.content =
            Cow::Owned(source[cursor.byte_index..].to_string());
        }

        if new_span.content.is_empty() {
          continue;
        }

        new_line.spans.push(new_span);
      }

      lines.push(new_line);
    }

    Text {
      alignment: text.alignment,
      style: text.style,
      lines,
    }
  }
}

#[derive(Clone, Copy, Debug, Default)]
struct RowCursor {
  byte_index: usize,
  line_index: usize,
  span_index: usize,
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    ratatui::{
      style::{Color, Style},
      text::Span,
    },
  };

  #[test]
  fn clip_to_bottom_returns_all_when_shorter() {
    let content = "line1\nline2";
    assert_eq!(
      App::clip_to_bottom(content, 5, 80),
      Text::raw(content.to_string())
    );
  }

  #[test]
  fn clip_to_bottom_limits_to_requested_lines() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2\nline3\nline4", 2, 80),
      Text::raw("line3\nline4".to_string())
    );
  }

  #[test]
  fn clip_to_bottom_handles_trailing_newlines() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2\n", 1, 80),
      Text::raw("line2\n")
    );
  }

  #[test]
  fn clip_to_bottom_with_zero_lines_returns_empty() {
    assert_eq!(App::clip_to_bottom("line1\nline2", 0, 80), Text::default());
  }

  #[test]
  fn clip_to_bottom_truncates_wrapped_lines() {
    assert_eq!(
      App::clip_to_bottom("1234567890abcdefghij", 2, 5),
      Text::raw("abcdefghij".to_string())
    );
  }

  #[test]
  fn clip_to_bottom_with_zero_columns_returns_empty() {
    assert_eq!(App::clip_to_bottom("line1\nline2", 5, 0), Text::default());
  }

  #[test]
  fn clip_to_bottom_preserves_color_in_wrapped_line() {
    let clipped = App::clip_to_bottom("\x1b[31mAAAAA\x1b[0m", 1, 2);

    let expected = Text::from(Line::from(Span::styled(
      "A",
      Style::default().fg(Color::Red),
    )));

    assert_eq!(clipped, expected);
  }

  #[test]
  fn clip_to_bottom_resets_styles_when_previous_lines_trimmed() {
    let clipped = App::clip_to_bottom("\x1b[32mline1\x1b[0m\nline2", 1, 80);

    let expected =
      Text::from(Line::from(Span::styled("line2", Style::reset())));

    assert_eq!(clipped, expected);
  }
}
