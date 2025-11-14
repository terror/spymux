use super::*;

pub(crate) fn run() -> Result {
  let current_pane_id = env::var("TMUX_PANE").ok();

  let mut panes = Tmux::list_spymux_instances()?;

  if let Some(current_pane_id) = current_pane_id {
    panes.retain(|pane| pane.id != current_pane_id);
  }

  if panes.is_empty() {
    bail!("no running spymux panes were found");
  }

  if panes.len() == 1 {
    Tmux::focus_pane(&panes[0])?;
    return Ok(());
  }

  if let Some(pane) = select_pane(&panes)? {
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
      writeln!(
        &mut stdin,
        "{}\t{}\t{}",
        pane.descriptor(),
        sanitize_path(&pane.path),
        pane.id
      )?;
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
    .filter(|line| !line.is_empty())
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sanitize_path_replaces_tabs() {
    assert_eq!(sanitize_path("/tmp\ttabs"), "/tmp tabs");
  }

  #[test]
  fn sanitize_path_handles_empty() {
    assert_eq!(sanitize_path(""), "-");
  }
}
