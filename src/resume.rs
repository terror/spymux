use {super::*, crate::tmux::SpymuxInstance, std::io::Write};

pub(crate) fn run() -> Result {
  let instances = Tmux::list_spymux_instances()?;

  if instances.is_empty() {
    bail!("no running spymux panes were found");
  }

  if let Some(instance) = select_instance(&instances)? {
    Tmux::focus_pane(&instance.pane)?;
  }

  Ok(())
}

fn select_instance(
  instances: &[SpymuxInstance],
) -> Result<Option<SpymuxInstance>> {
  let mut child = Command::new("fzf")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .context("failed to start fzf")?;

  {
    let mut stdin =
      child.stdin.take().context("failed to open stdin for fzf")?;

    for instance in instances {
      let path = sanitize_path(&instance.current_path);
      writeln!(
        &mut stdin,
        "{}\t{}\t{}",
        instance.pane.id, path, instance.pane.tmux_pane_id
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
    .filter(|s| !s.is_empty())
  else {
    return Ok(None);
  };

  let Some(pane_id) = line.rsplit('\t').next().filter(|s| !s.is_empty()) else {
    bail!("failed to parse selection from fzf");
  };

  let instance = instances
    .iter()
    .find(|instance| instance.pane.tmux_pane_id == pane_id)
    .ok_or_else(|| anyhow!("unable to locate pane {pane_id}"))?;

  Ok(Some(instance.clone()))
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
