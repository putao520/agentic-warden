//! Common utility functions used across the project

/// Format bytes into human readable string (B, KB, MB, GB, TB)
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format bytes into human readable string (alternative format)
/// This matches the format used in push.rs and pull.rs screens
pub fn format_bytes_alt(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_bytes_alt() {
        assert_eq!(format_bytes_alt(0), "0 bytes");
        assert_eq!(format_bytes_alt(512), "512 bytes");
        assert_eq!(format_bytes_alt(1024), "1.00 KB");
        assert_eq!(format_bytes_alt(1536), "1.50 KB");
        assert_eq!(format_bytes_alt(1048576), "1.00 MB");
        assert_eq!(format_bytes_alt(1073741824), "1.00 GB");
    }
}
