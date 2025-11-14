use super::*;

#[derive(Debug)]
pub(crate) struct App {
  config: Config,
  last_refresh: Instant,
  pane_regions: Vec<Rect>,
  selected_pane_id: Option<String>,
  terminal: TerminalGuard,
  tmux: Tmux,
}

impl App {
  const REFRESH_INTERVAL: Duration = Duration::from_millis(500);

  fn clip_to_bottom(
    content: &str,
    max_lines: usize,
    max_columns: usize,
    color_output: bool,
  ) -> Text<'static> {
    if max_lines == 0 || max_columns == 0 || content.is_empty() {
      return Text::default();
    }

    let parsed_text = content
      .into_text()
      .unwrap_or_else(|_| Text::raw(content.to_string()));

    let parsed_text = if color_output {
      parsed_text
    } else {
      Self::plain_text(parsed_text)
    };

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

  fn compute_pane_regions(area: Rect, pane_count: usize) -> Vec<Rect> {
    if pane_count == 0 {
      return Vec::new();
    }

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
      let column_constraints = vec![Constraint::Ratio(1, columns_u32); columns];

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

    pane_areas
  }

  fn ensure_selection(&mut self) {
    if self.tmux.panes.is_empty() {
      self.selected_pane_id = None;
      return;
    }

    if self.selected_pane().is_some() {
      return;
    }

    if let Some(pane) = self.tmux.panes.first() {
      self.selected_pane_id = Some(pane.id.clone());
    }
  }

  fn focus_pane(&mut self, pane: &Pane) -> Result {
    Tmux::focus_pane(pane)?;
    self.selected_pane_id = Some(pane.id.clone());
    Ok(())
  }

  fn handle_event(&mut self, event: Event) -> Result<Option<Action>> {
    match event {
      Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
          return Ok(Some(Action::Quit));
        }
        KeyCode::Char('h') | KeyCode::Left => {
          self.move_selection(Movement::Left)?;
        }
        KeyCode::Char('j') | KeyCode::Down => {
          self.move_selection(Movement::Down)?;
        }
        KeyCode::Char('k') | KeyCode::Up => {
          self.move_selection(Movement::Up)?;
        }
        KeyCode::Char('l') | KeyCode::Right => {
          self.move_selection(Movement::Right)?;
        }
        KeyCode::Enter => {
          if let Some(pane) = self.selected_pane() {
            return Ok(Some(Action::FocusPane(pane)));
          }
        }
        _ => {}
      },
      Event::Mouse(mouse_event) => {
        self.handle_mouse_event(mouse_event)?;
      }
      _ => {}
    }

    Ok(None)
  }

  fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> Result {
    if mouse_event.kind != MouseEventKind::Down(MouseButton::Left) {
      return Ok(());
    }

    let Some(pane_index) =
      self
        .pane_regions
        .iter()
        .enumerate()
        .find_map(|(index, rect)| {
          if Self::rect_contains(*rect, mouse_event.column, mouse_event.row) {
            Some(index)
          } else {
            None
          }
        })
    else {
      return Ok(());
    };

    self.select_pane_at_index(pane_index);

    Ok(())
  }

  fn line_is_empty(line: &Line<'_>) -> bool {
    if line.spans.is_empty() {
      return true;
    }

    line.spans.iter().all(|span| span.content.is_empty())
  }

  fn move_selection(&mut self, direction: Movement) -> Result {
    if self.tmux.panes.is_empty() {
      return Ok(());
    }

    self.ensure_selection();

    if self.pane_regions.len() != self.tmux.panes.len() {
      return Ok(());
    }

    let Some(selected_id) = self.selected_pane_id.as_deref() else {
      return Ok(());
    };

    let Some(current_index) = self
      .tmux
      .panes
      .iter()
      .position(|pane| pane.id == selected_id)
    else {
      return Ok(());
    };

    let Some(next_index) =
      Self::pane_in_direction(&self.pane_regions, current_index, direction)
    else {
      return Ok(());
    };

    self.select_pane_at_index(next_index);

    Ok(())
  }

  pub(crate) fn new(config: Config) -> Result<Self> {
    let terminal = TerminalGuard::new()?;

    let mut tmux = Tmux::new(config);

    if let Ok(pane_id) = env::var("TMUX_PANE") {
      tmux.exclude_pane_id(&pane_id);
    }

    tmux.capture()?;

    let selected_pane_id = tmux.panes.first().map(|pane| pane.id.clone());

    Ok(Self {
      config,
      pane_regions: Vec::new(),
      selected_pane_id,
      terminal,
      tmux,
      last_refresh: Instant::now(),
    })
  }

  fn pane_center(rect: Rect) -> (i32, i32) {
    (
      i32::from(rect.x) + i32::from(rect.width) / 2,
      i32::from(rect.y) + i32::from(rect.height) / 2,
    )
  }

  fn pane_in_direction(
    pane_regions: &[Rect],
    current_index: usize,
    direction: Movement,
  ) -> Option<usize> {
    let current_rect = pane_regions.get(current_index).copied()?;

    let (current_x, current_y) = Self::pane_center(current_rect);

    let mut best: Option<(i32, i32, usize)> = None;

    for (index, rect) in pane_regions.iter().copied().enumerate() {
      if index == current_index {
        continue;
      }

      let (candidate_x, candidate_y) = Self::pane_center(rect);

      let directional = match direction {
        Movement::Left if candidate_x < current_x => Some((
          current_x - candidate_x,
          (candidate_y - current_y).abs(),
          index,
        )),
        Movement::Right if candidate_x > current_x => Some((
          candidate_x - current_x,
          (candidate_y - current_y).abs(),
          index,
        )),
        Movement::Up if candidate_y < current_y => Some((
          current_y - candidate_y,
          (candidate_x - current_x).abs(),
          index,
        )),
        Movement::Down if candidate_y > current_y => Some((
          candidate_y - current_y,
          (candidate_x - current_x).abs(),
          index,
        )),
        _ => None,
      };

      let Some(candidate) = directional else {
        continue;
      };

      if best.is_none_or(|best_candidate| {
        candidate.0 < best_candidate.0
          || (candidate.0 == best_candidate.0 && candidate.1 < best_candidate.1)
      }) {
        best = Some(candidate);
      }
    }

    best.map(|(_, _, index)| index)
  }

  fn plain_text(mut text: Text<'static>) -> Text<'static> {
    text.style = Style::default();

    for line in &mut text.lines {
      line.style = Style::default();

      for span in &mut line.spans {
        span.style = Style::default();
      }
    }

    text
  }

  fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x
      && row >= rect.y
      && column < rect.x.saturating_add(rect.width)
      && row < rect.y.saturating_add(rect.height)
  }

  fn refresh_tmux(&mut self) -> Result {
    self.tmux.capture()?;
    self.ensure_selection();
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
      self.tick()?;

      let timeout = Self::REFRESH_INTERVAL
        .checked_sub(self.last_refresh.elapsed())
        .unwrap_or(Duration::from_millis(0));

      if event::poll(timeout)? {
        let event = event::read()?;

        if let Some(action) = self.handle_event(event)? {
          match action {
            Action::Quit => break,
            Action::FocusPane(pane) => {
              self.focus_pane(&pane)?;
            }
          }
        }
      }
    }

    Ok(())
  }

  fn select_pane_at_index(&mut self, pane_index: usize) {
    if let Some(pane) = self.tmux.panes.get(pane_index) {
      self.selected_pane_id = Some(pane.id.clone());
    }
  }

  fn selected_pane(&self) -> Option<Pane> {
    let selected_id = self.selected_pane_id.as_deref()?;

    self
      .tmux
      .panes
      .iter()
      .find(|pane| pane.id == selected_id)
      .cloned()
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

  fn tick(&mut self) -> Result {
    if self.last_refresh.elapsed() >= Self::REFRESH_INTERVAL {
      self.refresh_tmux()?;
    }

    let terminal = self.terminal.terminal_mut();

    terminal.draw(|frame| {
      let area = frame.area();

      if self.tmux.panes.is_empty() {
        self.pane_regions.clear();

        let widget = Paragraph::new("No tmux panes detected")
          .block(Block::default().title("tmux panes").borders(Borders::ALL));

        frame.render_widget(widget, area);

        return;
      }

      let pane_count = self.tmux.panes.len();
      let pane_areas = Self::compute_pane_regions(area, pane_count);

      self.pane_regions.clone_from(&pane_areas);

      for (pane_index, pane) in self.tmux.panes.iter().enumerate() {
        let pane_area = pane_areas[pane_index];

        let (inner_height, inner_width) = (
          pane_area.height.saturating_sub(2),
          pane_area.width.saturating_sub(2),
        );

        let (visible_lines, visible_columns) =
          (usize::from(inner_height), usize::from(inner_width));

        let clipped_content = Self::clip_to_bottom(
          &pane.content,
          visible_lines,
          visible_columns,
          self.config.color_output,
        );

        let mut block = Block::default()
          .title(pane.descriptor())
          .borders(Borders::ALL);

        if self
          .selected_pane_id
          .as_deref()
          .is_some_and(|id| id == pane.id)
        {
          block = block
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(Color::Cyan));
        } else {
          block = block.border_type(BorderType::Plain);
        }

        let widget = Paragraph::new(clipped_content)
          .wrap(Wrap { trim: false })
          .block(block);

        frame.render_widget(widget, pane_area);
      }
    })?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    ratatui::{style::Color, text::Span},
  };

  #[test]
  fn clip_to_bottom_returns_all_when_shorter() {
    let content = "line1\nline2";

    assert_eq!(
      App::clip_to_bottom(content, 5, 80, true),
      Text::raw(content.to_string())
    );
  }

  #[test]
  fn clip_to_bottom_limits_to_requested_lines() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2\nline3\nline4", 2, 80, true),
      Text::raw("line3\nline4".to_string())
    );
  }

  #[test]
  fn clip_to_bottom_handles_trailing_newlines() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2\n", 1, 80, true),
      Text::raw("line2\n")
    );
  }

  #[test]
  fn clip_to_bottom_with_zero_lines_returns_empty() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2", 0, 80, true),
      Text::default()
    );
  }

  #[test]
  fn clip_to_bottom_truncates_wrapped_lines() {
    assert_eq!(
      App::clip_to_bottom("1234567890abcdefghij", 2, 5, true),
      Text::raw("abcdefghij".to_string())
    );
  }

  #[test]
  fn clip_to_bottom_with_zero_columns_returns_empty() {
    assert_eq!(
      App::clip_to_bottom("line1\nline2", 5, 0, true),
      Text::default()
    );
  }

  #[test]
  fn clip_to_bottom_preserves_color_in_wrapped_line() {
    assert_eq!(
      App::clip_to_bottom("\x1b[31mAAAAA\x1b[0m", 1, 2, true),
      Text::from(Line::from(Span::styled(
        "A",
        Style::default().fg(Color::Red),
      )))
    );
  }

  #[test]
  fn clip_to_bottom_resets_styles_when_previous_lines_trimmed() {
    assert_eq!(
      App::clip_to_bottom("\x1b[32mline1\x1b[0m\nline2", 1, 80, true),
      Text::from(Line::from(Span::styled("line2", Style::reset())))
    );
  }

  #[test]
  fn clip_to_bottom_without_color_output_omits_styles() {
    assert_eq!(
      App::clip_to_bottom("\x1b[31mline\x1b[0m", 1, 80, false),
      Text::raw("line".to_string())
    );
  }

  #[test]
  fn collect_row_starts_wraps_long_line() {
    let text = Text::raw("abcdef".to_string());

    let starts = App::collect_row_starts(&text, 3, text.lines.len());

    let coordinates = starts
      .into_iter()
      .map(|cursor| (cursor.line_index, cursor.span_index, cursor.byte_index))
      .collect::<Vec<(usize, usize, usize)>>();

    assert_eq!(coordinates, vec![(0, 0, 0), (0, 0, 3)]);
  }

  #[test]
  fn line_is_empty_detects_content() {
    let mut line = Line::default();

    assert!(App::line_is_empty(&line));

    line.spans.push(Span::raw(""));
    assert!(App::line_is_empty(&line));

    line.spans.push(Span::raw("content"));
    assert!(!App::line_is_empty(&line));
  }

  #[test]
  fn plain_text_strips_styles() {
    let mut text = Text::from(Line::from(Span::styled(
      "styled",
      Style::default().fg(Color::Green).bg(Color::Blue),
    )));

    text.style = Style::default().fg(Color::Red);

    text.lines[0].style = Style::default().bg(Color::Yellow);

    let plain = App::plain_text(text);

    assert_eq!(plain.style, Style::default());
    assert_eq!(plain.lines[0].style, Style::default());
    assert_eq!(plain.lines[0].spans[0].style, Style::default());
  }

  #[test]
  fn renderable_line_count_ignores_trailing_empty_lines() {
    assert_eq!(
      App::renderable_line_count(&Text::from(vec![
        Line::from("line"),
        Line::default(),
        Line::from("")
      ])),
      1
    );
  }

  #[test]
  fn pane_in_direction_moves_right() {
    let pane_regions = vec![Rect::new(0, 0, 10, 5), Rect::new(12, 0, 10, 5)];

    assert_eq!(
      App::pane_in_direction(&pane_regions, 0, Movement::Right),
      Some(1)
    );
  }

  #[test]
  fn pane_in_direction_handles_missing_columns() {
    let pane_regions = vec![
      Rect::new(0, 0, 10, 5),
      Rect::new(12, 0, 10, 5),
      Rect::new(0, 8, 10, 5),
    ];

    assert_eq!(
      App::pane_in_direction(&pane_regions, 1, Movement::Down),
      Some(2)
    );
  }
}
