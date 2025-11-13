use {
  anyhow::{Result, bail},
  std::process::{Command, Output},
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pane {
  pub content: String,
  pub id: String,
  pub pane_index: usize,
  pub session: String,
  pub window: usize,
}

trait CommandRunner {
  fn run(&self, args: &[&str]) -> Result<Output>;
}

struct TmuxCommandRunner;

impl CommandRunner for TmuxCommandRunner {
  fn run(&self, args: &[&str]) -> Result<Output> {
    Ok(Command::new("tmux").args(args).output()?)
  }
}

#[derive(Debug)]
pub(crate) struct Tmux {
  panes: Vec<Pane>,
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
      bail!("invalid pane format: {}", line);
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
    list_panes_output: String,
    capture_outputs: BTreeMap<String, String>,
  }

  impl CommandRunner for MockCommandRunner {
    fn run(&self, args: &[&str]) -> Result<Output> {
      match args[0] {
        "list-panes" => Ok(Output {
          status: ExitStatus::default(),
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

          Ok(Output {
            status: ExitStatus::default(),
            stdout: content.as_bytes().to_vec(),
            stderr: vec![],
          })
        }
        _ => bail!("unexpected command"),
      }
    }
  }

  #[test]
  fn test_capture_single_pane() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs
      .insert("session1:0.0".to_string(), "Hello World\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\n".to_string(),
      capture_outputs,
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
  fn test_capture_multiple_panes() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert("session1:0.0".to_string(), "Pane 1\n".to_string());
    capture_outputs.insert("session1:0.1".to_string(), "Pane 2\n".to_string());
    capture_outputs.insert("session2:1.0".to_string(), "Pane 3\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\nsession1:0.1\nsession2:1.0\n"
        .to_string(),
      capture_outputs,
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 3);
    assert_eq!(tmux.panes[0].session, "session1");
    assert_eq!(tmux.panes[1].window, 0);
    assert_eq!(tmux.panes[2].pane_index, 0);
  }

  #[test]
  fn test_parse_pane_with_different_indices() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs
      .insert("mysession:5.3".to_string(), "Content\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: "mysession:5.3\n".to_string(),
      capture_outputs,
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes[0].session, "mysession");
    assert_eq!(tmux.panes[0].window, 5);
    assert_eq!(tmux.panes[0].pane_index, 3);
  }

  #[test]
  fn test_empty_pane_list() {
    let runner = MockCommandRunner {
      list_panes_output: "".to_string(),
      capture_outputs: BTreeMap::new(),
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 0);
  }

  #[test]
  fn test_skips_empty_lines() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert("session1:0.0".to_string(), "Content\n".to_string());

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\n\n\n".to_string(),
      capture_outputs,
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes.len(), 1);
  }

  #[test]
  fn test_multiline_content() {
    let mut capture_outputs = BTreeMap::new();

    capture_outputs.insert(
      "session1:0.0".to_string(),
      "Line 1\nLine 2\nLine 3\n".to_string(),
    );

    let runner = MockCommandRunner {
      list_panes_output: "session1:0.0\n".to_string(),
      capture_outputs,
    };

    let tmux = Tmux::capture_with_runner(&runner).unwrap();

    assert_eq!(tmux.panes[0].content, "Line 1\nLine 2\nLine 3\n");
  }
}
