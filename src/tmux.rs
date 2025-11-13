use super::*;

#[derive(Debug)]
pub(crate) struct Tmux {
  pub(crate) panes: Vec<Pane>,
}

impl Tmux {
  pub(crate) fn capture() -> Result<Self> {
    Self::capture_with_runner(&TmuxCommandRunner)
  }

  fn capture_with_runner(runner: &dyn CommandRunner) -> Result<Self> {
    let output = runner.run(&[
      "list-panes",
      "-a",
      "-F",
      "#{session_name}:#{window_index}.#{pane_index}",
    ])?;

    if !output.status.success() {
      bail!("failed to list tmux panes");
    }

    let pane_list = String::from_utf8(output.stdout)?;

    let mut panes = Vec::new();

    for line in pane_list.lines() {
      if line.is_empty() {
        continue;
      }

      panes.push(Self::parse_and_capture_pane(line, runner)?);
    }

    Ok(Self { panes })
  }

  fn parse_and_capture_pane(
    line: &str,
    runner: &dyn CommandRunner,
  ) -> Result<Pane> {
    let parts: Vec<&str> = line.split(':').collect();

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

    let content_output = runner.run(&["capture-pane", "-t", line, "-p"])?;

    if !content_output.status.success() {
      bail!("failed to capture pane output");
    }

    let content = String::from_utf8_lossy(&content_output.stdout).to_string();

    Ok(Pane {
      content,
      id: line.to_string(),
      pane_index,
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

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 0);
  }

  #[test]
  fn capture_single_pane() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs
      .insert("session1:0.0".to_string(), "Hello World\n".to_string());

    let runner = MockCommandRunner {
      capture_outputs,
      list_panes_output: String::from("session1:0.0\n"),
      ..Default::default()
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 1);

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Hello World\n".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
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
        "session1:0.0\nsession1:0.1\nsession2:1.0\n",
      ),
      ..Default::default()
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![
        Pane {
          content: "Pane 1\n".to_string(),
          id: "session1:0.0".to_string(),
          pane_index: 0,
          session: "session1".to_string(),
          window: 0,
        },
        Pane {
          content: "Pane 2\n".to_string(),
          id: "session1:0.1".to_string(),
          pane_index: 1,
          session: "session1".to_string(),
          window: 0,
        },
        Pane {
          content: "Pane 3\n".to_string(),
          id: "session2:1.0".to_string(),
          pane_index: 0,
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
      list_panes_output: "mysession:5.3\n".to_string(),
      capture_outputs,
      ..Default::default()
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Content\n".to_string(),
        id: "mysession:5.3".to_string(),
        pane_index: 3,
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
      list_panes_output: "session1:0.0\n\n\n".to_string(),
      capture_outputs,
      ..Default::default()
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Content\n".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
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
      list_panes_output: "session1:0.0\n".to_string(),
      capture_outputs,
      ..Default::default()
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(
      tmux.panes,
      vec![Pane {
        content: "Line 1\nLine 2\nLine 3\n".to_string(),
        id: "session1:0.0".to_string(),
        pane_index: 0,
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

    let err = Tmux::capture_with_runner(&runner).unwrap_err();

    assert!(err.to_string().contains("failed to list tmux panes"));
  }

  #[test]
  fn invalid_pane_format_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "not_a_valid_pane\n".to_string(),
      ..Default::default()
    };

    let err = Tmux::capture_with_runner(&runner).unwrap_err();

    assert!(err.to_string().contains("invalid pane format"));
  }

  #[test]
  fn invalid_window_pane_format_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "session1-0-0\n".to_string(),
      ..Default::default()
    };

    let err = Tmux::capture_with_runner(&runner).unwrap_err();

    assert!(err.to_string().contains("invalid pane format"));
  }

  #[test]
  fn invalid_window_index_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "session1:not_a_number.0\n".to_string(),
      ..Default::default()
    };

    let err = Tmux::capture_with_runner(&runner).unwrap_err();

    assert!(err.downcast_ref::<std::num::ParseIntError>().is_some());
  }

  #[test]
  fn invalid_pane_index_returns_error() {
    let runner = MockCommandRunner {
      list_panes_output: "session1:0.not_a_number\n".to_string(),
      ..Default::default()
    };

    let err = Tmux::capture_with_runner(&runner).unwrap_err();

    assert!(err.downcast_ref::<std::num::ParseIntError>().is_some());
  }

  #[test]
  fn capture_pane_command_failure() {
    let mut capture_successes = BTreeMap::new();
    capture_successes.insert("session1:0.0".to_string(), false);

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\n".to_string(),
      capture_successes,
      ..Default::default()
    };

    let err = Tmux::capture_with_runner(&runner).unwrap_err();

    assert!(err.to_string().contains("failed to capture pane output"));
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

    let err = Tmux::capture_with_runner(&InvalidUtf8Runner).unwrap_err();

    assert!(err.downcast_ref::<std::string::FromUtf8Error>().is_some());
  }
}
