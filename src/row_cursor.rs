#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RowCursor {
  pub(crate) byte_index: usize,
  pub(crate) line_index: usize,
  pub(crate) span_index: usize,
}
