//! Byte-size formatting helpers used by the migrate commands and the
//! S3 client.

/// Format `bytes` as a decimal-unit size string.
///
/// Uses SI units (`B`, `kB`, `MB`, `GB`, `TB`) with one decimal place
/// for any unit above bytes. Values below `1_000` are rendered as
/// integer bytes (e.g. `999` → `"999 B"`).
#[must_use]
pub(crate) fn format_bytes(bytes: u64) -> String {
  const UNITS: [&str; 4] = ["kB", "MB", "GB", "TB"];
  if bytes < 1_000 {
    return format!("{bytes} B");
  }
  let mut value = bytes as f64 / 1_000.0;
  let mut unit_index = 0;
  while value >= 1_000.0 && unit_index < UNITS.len() - 1 {
    value /= 1_000.0;
    unit_index += 1;
  }
  format!("{value:.1} {}", UNITS[unit_index])
}

#[cfg(test)]
mod tests {
  use super::format_bytes;

  #[test]
  fn matches_decimal_shape() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(999), "999 B");
    assert_eq!(format_bytes(1_000), "1.0 kB");
    assert_eq!(format_bytes(1_500), "1.5 kB");
    assert_eq!(format_bytes(1_500_000), "1.5 MB");
    assert_eq!(format_bytes(1_500_000_000), "1.5 GB");
    assert_eq!(format_bytes(1_500_000_000_000), "1.5 TB");
  }
}
