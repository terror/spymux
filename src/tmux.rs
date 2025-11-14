use super::*;

#[derive(Debug, Default)]
pub(crate) struct Tmux {
  pub(crate) excluded_pane_ids: Vec<String>,
  pub(crate) include_escape_codes: bool,
  pub(crate) panes: Vec<Pane>,
}

impl Tmux {
  pub(crate) fn capture(&mut self) -> Result {
    self.capture_with_runner(&TmuxCommandRunner)
  }

  fn capture_entry(
    &self,
    mut pane: Pane,
    runner: &dyn CommandRunner,
  ) -> Result<Pane> {
    let descriptor = pane.descriptor();

    let mut capture_cmd = vec!["capture-pane", "-t", descriptor.as_str(), "-p"];

    if self.include_escape_codes {
      capture_cmd.push("-e");
    }

    let content_output = runner.run(&capture_cmd)?;

    if !content_output.status.success() {
      bail!("failed to capture pane output");
    }

    let content = String::from_utf8_lossy(&content_output.stdout).to_string();

    pane.content = content;

    Ok(pane)
  }

  fn capture_with_runner(&mut self, runner: &dyn CommandRunner) -> Result {
    let excluded = &self.excluded_pane_ids;

    self.panes = Self::list_panes(runner)?
      .into_iter()
      .filter(|pane| !excluded.contains(&pane.id))
      .map(|pane| self.capture_entry(pane, runner))
      .collect::<Result<Vec<_>>>()?;

    Ok(())
  }

  pub(crate) fn exclude_pane_id(&mut self, pane_id: &str) {
    self.panes.retain(|pane| pane.id != pane_id);
    self.excluded_pane_ids.push(pane_id.to_string());
  }

  pub(crate) fn focus_pane(pane: &Pane) -> Result {
    Self::focus_pane_with_runner(pane, &TmuxCommandRunner)
  }

  fn focus_pane_with_runner(pane: &Pane, runner: &dyn CommandRunner) -> Result {
    Self::select_window_with_runner(
      &format!("{}:{}", pane.session, pane.window_index),
      runner,
    )?;

    Self::select_pane_with_runner(&pane.id, runner)
  }

  fn list_panes(runner: &dyn CommandRunner) -> Result<Vec<Pane>> {
    let output = runner.run(&["list-panes", "-a", "-F", Pane::format()])?;

    if !output.status.success() {
      bail!("failed to list tmux panes");
    }

    let pane_list = String::from_utf8(output.stdout)?;

    pane_list
      .lines()
      .filter(|line| !line.is_empty())
      .map(|line| serde_json::from_str(line).map_err(Into::into))
      .collect()
  }

  pub(crate) fn list_spymux_instances() -> Result<Vec<Pane>> {
    Self::list_spymux_instances_with_runner(&TmuxCommandRunner)
  }

  fn list_spymux_instances_with_runner(
    runner: &dyn CommandRunner,
  ) -> Result<Vec<Pane>> {
    Ok(
      Self::list_panes(runner)?
        .into_iter()
        .filter(|pane| pane.command.trim().eq_ignore_ascii_case("spymux"))
        .collect(),
    )
  }

  pub(crate) fn new(config: Config) -> Self {
    Self {
      excluded_pane_ids: Vec::new(),
      include_escape_codes: config.color_output,
      panes: Vec::new(),
    }
  }

  fn select_pane_with_runner(
    pane_id: &str,
    runner: &dyn CommandRunner,
  ) -> Result {
    let output = runner.run(&["select-pane", "-t", pane_id])?;

    if !output.status.success() {
      bail!("failed to select tmux pane");
    }

    Ok(())
  }

  fn select_window_with_runner(
    target: &str,
    runner: &dyn CommandRunner,
  ) -> Result {
    let output = runner.run(&["select-window", "-t", target])?;

    if !output.status.success() {
      bail!("failed to select tmux window");
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    serde_json::json,
    std::{cell::RefCell, collections::BTreeMap, process::ExitStatus},
  };

  struct MockCommandRunner {
    capture_outputs: BTreeMap<String, String>,
    capture_successes: BTreeMap<String, bool>,
    list_panes_output: String,
    list_panes_success: bool,
    select_pane_success: bool,
    select_window_success: bool,
    selected_panes: RefCell<Vec<String>>,
    selected_windows: RefCell<Vec<String>>,
  }

  impl Default for MockCommandRunner {
    fn default() -> Self {
      Self {
        capture_outputs: BTreeMap::new(),
        capture_successes: BTreeMap::new(),
        list_panes_output: String::new(),
        list_panes_success: true,
        select_pane_success: true,
        select_window_success: true,
        selected_panes: RefCell::new(Vec::new()),
        selected_windows: RefCell::new(Vec::new()),
      }
    }
  }

  impl CommandRunner for MockCommandRunner {
    fn run(&self, arguments: &[&str]) -> Result<Output> {
      match arguments[0] {
        "list-panes" => Ok(Output {
          status: exit_status(self.list_panes_success),
          stdout: self.list_panes_output.as_bytes().to_vec(),
          stderr: vec![],
        }),
        "capture-pane" => {
          let pane_id = arguments[2];

          let content = self
            .capture_outputs
            .get(pane_id)
            .unwrap_or(&String::new())
            .clone();

          let success = *self.capture_successes.get(pane_id).unwrap_or(&true);

          Ok(Output {
            status: exit_status(success),
            stdout: content.as_bytes().to_vec(),
            stderr: vec![],
          })
        }
        "select-pane" => {
          let target = arguments[2].to_string();

          self.selected_panes.borrow_mut().push(target);

          Ok(Output {
            status: exit_status(self.select_pane_success),
            stdout: vec![],
            stderr: vec![],
          })
        }
        "select-window" => {
          let target = arguments[2].to_string();

          self.selected_windows.borrow_mut().push(target);

          Ok(Output {
            status: exit_status(self.select_window_success),
            stdout: vec![],
            stderr: vec![],
          })
        }
        _ => bail!("unexpected command"),
      }
    }
  }

  impl MockCommandRunner {
    fn selected_panes(&self) -> Vec<String> {
      self.selected_panes.borrow().clone()
    }

    fn selected_windows(&self) -> Vec<String> {
      self.selected_windows.borrow().clone()
    }
  }

  fn pane(
    session: &str,
    window_index: usize,
    index: usize,
    id: &str,
    command: &str,
    path: &str,
  ) -> String {
    json!({
      "command": command,
      "id": id,
      "index": index,
      "path": path,
      "session": session,
      "window_index": window_index,
    })
    .to_string()
  }

  #[cfg(unix)]
  fn exit_status(success: bool) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    if success {
      ExitStatus::from_raw(0)
    } else {
      ExitStatus::from_raw(1)
    }
  }

  #[cfg(windows)]
  fn exit_status(success: bool) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    if success {
      ExitStatus::from_raw(0)
    } else {
      ExitStatus::from_raw(1)
    }
  }

  #[cfg(not(any(unix, windows)))]
  fn exit_status(success: bool) -> ExitStatus {
    if success {
      ExitStatus::default()
    } else {
      panic!("unsupported platform for tests");
    }
  }

  #[test]
  fn empty_pane_list() {
    let runner = MockCommandRunner {
      list_panes_output: String::new(),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 0);
  }

  #[test]
  fn capture_single_pane() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs
      .insert("session1:0.0".to_string(), "Hello World\n".to_string());

    let runner = MockCommandRunner {
      capture_outputs,
      list_panes_output: format!("{}\n", pane("session1", 0, 0, "%0", "", "")),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 1);

    assert_eq!(
      tmux.panes,
      vec![Pane {
        command: String::new(),
        content: "Hello World\n".to_string(),
        index: 0,
        path: String::new(),
        session: "session1".to_string(),
        id: "%0".to_string(),
        window_index: 0,
      }]
    );
  }

  #[test]
  fn capture_multiple_panes() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert("session1:0.0".to_string(), "Pane 1\n".to_string());
    capture_outputs.insert("session1:0.1".to_string(), "Pane 2\n".to_string());
    capture_outputs.insert("session2:1.0".to_string(), "Pane 3\n".to_string());

    let runner = MockCommandRunner {
      capture_outputs,
      list_panes_output: format!(
        "{}\n{}\n{}\n",
        pane("session1", 0, 0, "%0", "", ""),
        pane("session1", 0, 1, "%1", "", ""),
        pane("session2", 1, 0, "%2", "", "")
      ),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![
        Pane {
          command: String::new(),
          content: "Pane 1\n".to_string(),
          index: 0,
          path: String::new(),
          session: "session1".to_string(),
          id: "%0".to_string(),
          window_index: 0,
        },
        Pane {
          command: String::new(),
          content: "Pane 2\n".to_string(),
          index: 1,
          path: String::new(),
          session: "session1".to_string(),
          id: "%1".to_string(),
          window_index: 0,
        },
        Pane {
          command: String::new(),
          content: "Pane 3\n".to_string(),
          index: 0,
          path: String::new(),
          session: "session2".to_string(),
          id: "%2".to_string(),
          window_index: 1,
        },
      ]
    );
  }

  #[test]
  fn capture_skips_excluded_panes() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert("session1:0.0".to_string(), "Pane 1\n".to_string());
    capture_outputs.insert("session1:0.1".to_string(), "Pane 2\n".to_string());

    let runner = MockCommandRunner {
      capture_outputs,
      list_panes_output: format!(
        "{}\n{}\n",
        pane("session1", 0, 0, "%0", "", ""),
        pane("session1", 0, 1, "%1", "", "")
      ),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.exclude_pane_id("%1");
    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        command: String::new(),
        content: "Pane 1\n".to_string(),
        index: 0,
        path: String::new(),
        session: "session1".to_string(),
        id: "%0".to_string(),
        window_index: 0,
      }]
    );
  }

  #[test]
  fn parse_pane_with_different_indices() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs
      .insert("mysession:5.3".to_string(), "Content\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: format!(
        "{}\n",
        pane("mysession", 5, 3, "%10", "", "")
      ),
      capture_outputs,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        command: String::new(),
        content: "Content\n".to_string(),
        index: 3,
        path: String::new(),
        session: "mysession".to_string(),
        id: "%10".to_string(),
        window_index: 5,
      }]
    );
  }

  #[test]
  fn skips_empty_lines() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert("session1:0.0".to_string(), "Content\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: format!(
        "{}\n\n\n",
        pane("session1", 0, 0, "%0", "", "")
      ),
      capture_outputs,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        command: String::new(),
        content: "Content\n".to_string(),
        index: 0,
        path: String::new(),
        session: "session1".to_string(),
        id: "%0".to_string(),
        window_index: 0,
      }]
    );
  }

  #[test]
  fn multiline_content() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert(
      "session1:0.0".to_string(),
      "Line 1\nLine 2\nLine 3\n".to_string(),
    );

    let runner = MockCommandRunner {
      list_panes_output: format!("{}\n", pane("session1", 0, 0, "%0", "", "")),
      capture_outputs,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        command: String::new(),
        content: "Line 1\nLine 2\nLine 3\n".to_string(),
        index: 0,
        path: String::new(),
        session: "session1".to_string(),
        id: "%0".to_string(),
        window_index: 0,
      }]
    );
  }

  #[test]
  fn exclude_pane_id_removes_matching_entry() {
    let mut tmux = Tmux {
      panes: vec![
        Pane {
          command: String::new(),
          content: "one".to_string(),
          index: 0,
          path: String::new(),
          session: "session1".to_string(),
          id: "%0".to_string(),
          window_index: 0,
        },
        Pane {
          command: String::new(),
          content: "two".to_string(),
          index: 1,
          path: String::new(),
          session: "session1".to_string(),
          id: "%1".to_string(),
          window_index: 0,
        },
      ],
      ..Default::default()
    };

    tmux.exclude_pane_id("%1");

    assert_eq!(
      tmux.panes,
      vec![Pane {
        command: String::new(),
        content: "one".to_string(),
        index: 0,
        path: String::new(),
        session: "session1".to_string(),
        id: "%0".to_string(),
        window_index: 0,
      }]
    );
  }

  #[test]
  fn select_pane_with_runner_invokes_tmux() {
    let runner = MockCommandRunner::default();

    Tmux::select_pane_with_runner("%42", &runner).unwrap();

    assert_eq!(runner.selected_panes(), vec!["%42".to_string()]);
  }

  #[test]
  fn select_pane_with_runner_errors_on_failure() {
    let runner = MockCommandRunner {
      select_pane_success: false,
      ..Default::default()
    };

    assert_eq!(
      Tmux::select_pane_with_runner("%1", &runner)
        .unwrap_err()
        .to_string(),
      "failed to select tmux pane"
    );
  }

  #[test]
  fn focus_pane_with_runner_selects_window_and_pane() {
    let runner = MockCommandRunner::default();

    let pane = Pane {
      command: String::new(),
      content: String::new(),
      index: 2,
      path: String::new(),
      session: "mysession".to_string(),
      id: "%12".to_string(),
      window_index: 3,
    };

    Tmux::focus_pane_with_runner(&pane, &runner).unwrap();

    assert_eq!(runner.selected_windows(), vec!["mysession:3".to_string()]);
    assert_eq!(runner.selected_panes(), vec!["%12".to_string()]);
  }

  #[test]
  fn focus_pane_with_runner_propagates_window_errors() {
    let runner = MockCommandRunner {
      select_window_success: false,
      ..Default::default()
    };

    let pane = Pane {
      command: String::new(),
      content: String::new(),
      index: 0,
      path: String::new(),
      session: "mysession".to_string(),
      id: "%3".to_string(),
      window_index: 1,
    };

    assert_eq!(
      Tmux::focus_pane_with_runner(&pane, &runner)
        .unwrap_err()
        .to_string(),
      "failed to select tmux window"
    );
  }

  #[test]
  fn list_spymux_instances_with_runner_filters_entries() {
    let runner = MockCommandRunner {
      list_panes_output: format!(
        "{}\n{}\n{}\n",
        pane("session1", 0, 0, "%0", "spymux", "/home/project"),
        pane("session1", 0, 1, "%1", "bash", "/home/other"),
        pane("session1", 0, 2, "%2", "bash", "/home/skip")
      ),
      ..Default::default()
    };

    let panes = Tmux::list_spymux_instances_with_runner(&runner).unwrap();

    assert_eq!(panes.len(), 1);

    assert_eq!(
      panes,
      vec![Pane {
        command: "spymux".to_string(),
        content: String::new(),
        index: 0,
        path: "/home/project".to_string(),
        session: "session1".to_string(),
        id: "%0".to_string(),
        window_index: 0,
      }]
    );
  }

  #[test]
  fn list_panes_command_failure() {
    let runner = MockCommandRunner {
      list_panes_success: false,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "failed to list tmux panes"
    );
  }

  #[test]
  fn invalid_pane_format_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "not_a_valid_json\n".to_string(),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "expected ident at line 1 column 2"
    );
  }

  #[test]
  fn invalid_window_pane_format_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: format!(
        "{}\n",
        json!({
          "command": "",
          "id": "%0",
          "path": "",
          "session": "session1",
          "window_index": 0,
        })
      ),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "missing field `index` at line 1 column 72"
    );
  }

  #[test]
  fn invalid_window_index_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: format!(
        "{}\n",
        json!({
          "command": "",
          "pane_id": "%0",
          "index": 0,
          "path": "",
          "session": "session1",
          "window_index": "not_a_number",
        })
      ),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "invalid type: string \"not_a_number\", expected usize at line 1 column 99"
    );
  }

  #[test]
  fn invalid_index_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: format!(
        "{}\n",
        json!({
          "command": "",
          "pane_id": "%0",
          "index": "not_a_number",
          "path": "",
          "session": "session1",
          "window_index": 0,
        })
      ),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "invalid type: string \"not_a_number\", expected usize at line 1 column 36"
    );
  }

  #[test]
  fn capture_pane_command_failure() {
    let mut capture_successes = BTreeMap::new();

    capture_successes.insert("session1:0.0".to_string(), false);

    let runner = MockCommandRunner {
      list_panes_output: format!("{}\n", pane("session1", 0, 0, "%0", "", "")),
      capture_successes,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "failed to capture pane output"
    );
  }

  #[test]
  fn invalid_utf8_in_list_output_propagates_error() {
    struct InvalidUtf8Runner;

    impl CommandRunner for InvalidUtf8Runner {
      fn run(&self, args: &[&str]) -> Result<Output> {
        match args[0] {
          "list-panes" => Ok(Output {
            status: exit_status(true),
            stdout: vec![0xf0, 0x28, 0x8c, 0x28],
            stderr: vec![],
          }),
          _ => bail!("unexpected command"),
        }
      }
    }

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux
        .capture_with_runner(&InvalidUtf8Runner)
        .unwrap_err()
        .to_string(),
      "invalid utf-8 sequence of 1 bytes from index 0"
    );
  }
}
