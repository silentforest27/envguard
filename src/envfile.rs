use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn load_dotenv(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    if !path.exists() {
        return Ok(map);
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    for (lineno, raw) in text.lines().enumerate() {
        let mut line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export") {
            let trimmed = rest.trim_start();
            if !trimmed.is_empty() && trimmed.len() < rest.len() {
                line = trimmed;
            }
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: missing '='", lineno + 1))?;
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("line {}: empty key", lineno + 1));
        }
        let mut value = value.trim().to_string();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len() - 1].to_string();
        }
        map.insert(key.to_string(), value);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_basic() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-test.env");
        let mut f = fs::File::create(&p).unwrap();
        writeln!(f, "FOO=bar").unwrap();
        writeln!(f, "# comment").unwrap();
        writeln!(f, "export BAZ=\"qux\"").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "bar");
        assert_eq!(m.get("BAZ").unwrap(), "qux");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_file_ok() {
        let p = std::env::temp_dir().join("envguard-nope.env");
        let _ = fs::remove_file(&p);
        let m = load_dotenv(&p).unwrap();
        assert!(m.is_empty());
    }

    #[test]
    fn export_with_extra_whitespace() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-export-whitespace.env");
        let mut f = fs::File::create(&p).unwrap();
        writeln!(f, "export   FOO=bar").unwrap();
        writeln!(f, "export\tBAZ=qux").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "bar");
        assert_eq!(m.get("BAZ").unwrap(), "qux");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn strips_byte_order_mark() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-bom.env");
        fs::write(&p, "\u{feff}FOO=bar\n").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "bar");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn export_prefix_requires_word_boundary() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-export-boundary.env");
        fs::write(&p, "exportFOO=bar\nexport BAR=baz\n").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("exportFOO").unwrap(), "bar");
        assert_eq!(m.get("BAR").unwrap(), "baz");
        assert!(!m.contains_key("FOO"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn quoted_value_with_spaces() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-quoted-spaces.env");
        fs::write(&p, "FOO=\"hello world\"\nBAR='x y'\n").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "hello world");
        assert_eq!(m.get("BAR").unwrap(), "x y");
        let _ = fs::remove_file(&p);
    }
}
