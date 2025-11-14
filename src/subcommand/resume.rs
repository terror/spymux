use super::*;

pub(crate) fn run() -> Result {
  let current_dir =
    env::current_dir().context("failed to determine current directory")?;

  let current_dir = fs::canonicalize(&current_dir).unwrap_or(current_dir);

  let panes = Tmux::list_spymux_instances()?;

  let candidates = panes
    .into_iter()
    .filter(|pane| !is_current_directory(&current_dir, pane.path.as_str()))
    .collect::<Vec<_>>();

  if candidates.is_empty() {
    bail!("no running spymux panes were found");
  }

  if candidates.len() == 1 {
    Tmux::focus_pane(&candidates[0])?;
    return Ok(());
  }

  if let Some(pane) = select_pane(&candidates)? {
    Tmux::focus_pane(&pane)?;
  }

  Ok(())
}

fn select_pane(panes: &[Pane]) -> Result<Option<Pane>> {
  let mut child = Command::new("fzf")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .context("failed to start fzf")?;

  {
    let mut stdin =
      child.stdin.take().context("failed to open stdin for fzf")?;

    for pane in panes {
      let path = sanitize_path(&pane.path);

      writeln!(&mut stdin, "{}\t{}\t{}", pane.descriptor(), path, pane.id)?;
    }
  }

  let output = child.wait_with_output()?;

  if !output.status.success() {
    return Ok(None);
  }

  let selection = String::from_utf8(output.stdout)?;

  let Some(line) = selection
    .lines()
    .next()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  else {
    return Ok(None);
  };

  let Some(pane_id) = line.rsplit('\t').next().filter(|s| !s.is_empty()) else {
    bail!("failed to parse selection from fzf");
  };

  let pane = panes
    .iter()
    .find(|pane| pane.id == pane_id)
    .ok_or_else(|| anyhow!("unable to locate pane {pane_id}"))?;

  Ok(Some(pane.clone()))
}

fn sanitize_path(path: &str) -> String {
  if path.is_empty() {
    return "-".to_string();
  }

  path.replace('\t', " ")
}

fn is_current_directory(current_dir: &Path, candidate: &str) -> bool {
  if candidate.is_empty() {
    return false;
  }

  let candidate_path = Path::new(candidate);

  if let Ok(canonical_candidate) = fs::canonicalize(candidate_path) {
    return canonical_candidate == current_dir;
  }

  candidate_path == current_dir
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    std::{env, fs},
  };

  #[test]
  fn sanitize_path_replaces_tabs() {
    assert_eq!(sanitize_path("/tmp\ttabs"), "/tmp tabs");
  }

  #[test]
  fn sanitize_path_handles_empty() {
    assert_eq!(sanitize_path(""), "-");
  }

  #[test]
  fn current_directory_comparison_skips_empty() {
    assert!(!is_current_directory(&env::temp_dir(), ""));
  }

  #[test]
  fn current_directory_comparison_matches() {
    let cwd = env::temp_dir();

    let canonical_cwd = fs::canonicalize(&cwd).unwrap_or(cwd.clone());

    assert!(is_current_directory(
      &canonical_cwd,
      &canonical_cwd.to_string_lossy()
    ));
  }
}
