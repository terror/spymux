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

  fn capture_with_runner(&mut self, runner: &dyn CommandRunner) -> Result {
    const FORMAT: &str =
      "#{session_name}:#{window_index}.#{pane_index}\t#{pane_id}";

    let output = runner.run(&["list-panes", "-a", "-F", FORMAT])?;

    if !output.status.success() {
      bail!("failed to list tmux panes");
    }

    let pane_list = String::from_utf8(output.stdout)?;

    let mut panes = Vec::new();

    for line in pane_list.lines() {
      if line.is_empty() {
        continue;
      }

      let pane = self.parse_and_capture_pane(line, runner)?;

      if self.excluded_pane_ids.contains(&pane.tmux_pane_id) {
        continue;
      }

      panes.push(pane);
    }

    self.panes = panes;

    Ok(())
  }

  pub(crate) fn exclude_pane_id(&mut self, pane_id: &str) {
    self.panes.retain(|pane| pane.tmux_pane_id != pane_id);
    self.excluded_pane_ids.push(pane_id.to_string());
  }

  pub(crate) fn new(config: Config) -> Self {
    Self {
      excluded_pane_ids: Vec::new(),
      include_escape_codes: config.color_output,
      panes: Vec::new(),
    }
  }

  fn parse_and_capture_pane(
    &self,
    line: &str,
    runner: &dyn CommandRunner,
  ) -> Result<Pane> {
    let Some((descriptor, pane_id)) = line.split_once('\t') else {
      bail!("invalid pane format: {line}");
    };

    let parts: Vec<&str> = descriptor.split(':').collect();

    if parts.len() != 2 {
      bail!("invalid pane format: {line}");
    }

    let session = parts[0].to_string();

    let window_pane = parts[1].split('.').collect::<Vec<&str>>();

    if window_pane.len() != 2 {
      bail!("invalid window.pane format: {}", parts[1]);
    }

    let (window, pane_index) = (
      window_pane[0].parse::<usize>()?,
      window_pane[1].parse::<usize>()?,
    );

    let mut capture_cmd = vec!["capture-pane", "-t", descriptor, "-p"];

    if self.include_escape_codes {
      capture_cmd.push("-e");
    }

    let content_output = runner.run(&capture_cmd)?;

    if !content_output.status.success() {
      bail!("failed to capture pane output");
    }

    let content = String::from_utf8_lossy(&content_output.stdout).to_string();

    Ok(Pane {
      content,
      id: descriptor.to_string(),
      pane_index,
      tmux_pane_id: pane_id.to_string(),
      session,
      window,
    })
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    std::{collections::BTreeMap, process::ExitStatus},
  };

  struct MockCommandRunner {
    capture_outputs: BTreeMap<String, String>,
    capture_successes: BTreeMap<String, bool>,
    list_panes_output: String,
    list_panes_success: bool,
  }

  impl Default for MockCommandRunner {
    fn default() -> Self {
      Self {
        capture_outputs: BTreeMap::new(),
        capture_successes: BTreeMap::new(),
        list_panes_output: String::new(),
        list_panes_success: true,
      }
    }
  }

  impl CommandRunner for MockCommandRunner {
    fn run(&self, args: &[&str]) -> Result<Output> {
      match args[0] {
        "list-panes" => Ok(Output {
          status: exit_status(self.list_panes_success),
          stdout: self.list_panes_output.as_bytes().to_vec(),
          stderr: vec![],
        }),
        "capture-pane" => {
          let pane_id = args[2];

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
        _ => bail!("unexpected command"),
      }
    }
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
      list_panes_output: String::from("session1:0.0\t%0\n"),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 1);

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Hello World\n".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
        tmux_pane_id: "%0".to_string(),
        session: "session1".to_string(),
        window: 0,
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
      list_panes_output: String::from(
        "session1:0.0\t%0\nsession1:0.1\t%1\nsession2:1.0\t%2\n",
      ),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![
        Pane {
          content: "Pane 1\n".to_string(),
          id: "session1:0.0".to_string(),
          pane_index: 0,
          tmux_pane_id: "%0".to_string(),
          session: "session1".to_string(),
          window: 0,
        },
        Pane {
          content: "Pane 2\n".to_string(),
          id: "session1:0.1".to_string(),
          pane_index: 1,
          tmux_pane_id: "%1".to_string(),
          session: "session1".to_string(),
          window: 0,
        },
        Pane {
          content: "Pane 3\n".to_string(),
          id: "session2:1.0".to_string(),
          pane_index: 0,
          tmux_pane_id: "%2".to_string(),
          session: "session2".to_string(),
          window: 1,
        },
      ]
    );
  }

  #[test]
  fn parse_pane_with_different_indices() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs
      .insert("mysession:5.3".to_string(), "Content\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: "mysession:5.3\t%10\n".to_string(),
      capture_outputs,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Content\n".to_string(),
        id: "mysession:5.3".to_string(),
        pane_index: 3,
        tmux_pane_id: "%10".to_string(),
        session: "mysession".to_string(),
        window: 5,
      }]
    );
  }

  #[test]
  fn skips_empty_lines() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert("session1:0.0".to_string(), "Content\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\t%0\n\n\n".to_string(),
      capture_outputs,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Content\n".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
        tmux_pane_id: "%0".to_string(),
        session: "session1".to_string(),
        window: 0,
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
      list_panes_output: "session1:0.0\t%0\n".to_string(),
      capture_outputs,
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    tmux.capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Line 1\nLine 2\nLine 3\n".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
        tmux_pane_id: "%0".to_string(),
        session: "session1".to_string(),
        window: 0,
      }]
    );
  }

  #[test]
  fn exclude_pane_id_removes_matching_entry() {
    let mut tmux = Tmux {
      panes: vec![
        Pane {
          content: "one".to_string(),
          id: "session1:0.0".to_string(),
          pane_index: 0,
          tmux_pane_id: "%0".to_string(),
          session: "session1".to_string(),
          window: 0,
        },
        Pane {
          content: "two".to_string(),
          id: "session1:0.1".to_string(),
          pane_index: 1,
          tmux_pane_id: "%1".to_string(),
          session: "session1".to_string(),
          window: 0,
        },
      ],
      ..Default::default()
    };

    tmux.exclude_pane_id("%1");

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "one".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
        tmux_pane_id: "%0".to_string(),
        session: "session1".to_string(),
        window: 0,
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
      list_panes_output: "not_a_valid_pane\n".to_string(),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "invalid pane format: not_a_valid_pane"
    );
  }

  #[test]
  fn invalid_window_pane_format_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "session1-0-0\n".to_string(),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "invalid pane format: session1-0-0"
    );
  }

  #[test]
  fn invalid_window_index_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "session1:not_a_number.0\t%0\n".to_string(),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "invalid digit found in string"
    );
  }

  #[test]
  fn invalid_pane_index_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "session1:0.not_a_number\t%0\n".to_string(),
      ..Default::default()
    };

    let mut tmux = Tmux::new(Config::default());

    assert_eq!(
      tmux.capture_with_runner(&runner).unwrap_err().to_string(),
      "invalid digit found in string"
    );
  }

  #[test]
  fn capture_pane_command_failure() {
    let mut capture_successes = BTreeMap::new();

    capture_successes.insert("session1:0.0".to_string(), false);

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\t%0\n".to_string(),
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
