use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn load_dotenv(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    if !path.exists() {
        return Ok(map);
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let text = text.strip_prefix(char::from_u32(0xFEFF).unwrap()).unwrap_or(&text);
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
        let value = parse_value(value.trim(), lineno + 1)?;
        map.insert(key.to_string(), value);
    }
    Ok(map)
}

fn parse_value(input: &str, lineno: usize) -> Result<String, String> {
    if input.is_empty() {
        return Ok(String::new());
    }

    let first = input.chars().next().unwrap();
    if first == char::from(39) {
        let rest = &input[1..];
        let end = rest
            .find(char::from(39))
            .ok_or_else(|| format!("line {}: unterminated single-quoted value", lineno))?;
        let content = &rest[..end];
        let trailing = rest[end + 1..].trim();
        if !trailing.is_empty() && !trailing.starts_with('#') {
            return Err(format!(
                "line {}: unexpected trailing data after quoted value",
                lineno
            ));
        }
        Ok(content.to_string())
    } else if first == char::from(34) {
        let mut result = String::new();
        let mut chars = input[1..].chars();
        let mut closed = false;
        let mut trailing = String::new();

        while let Some(c) = chars.next() {
            if c == char::from(92) {
                match chars.next() {
                    Some('n') => result.push(char::from(10)),
                    Some('t') => result.push(char::from(9)),
                    Some('r') => result.push(char::from(13)),
                    Some('"') => result.push(char::from(34)),
                    Some(x) if x == char::from(92) => result.push(char::from(92)),
                    Some(other) => {
                        result.push(char::from(92));
                        result.push(other);
                    }
                    None => {
                        return Err(format!("line {}: unterminated escape in quoted value", lineno))
                    }
                }
            } else if c == char::from(34) {
                closed = true;
                trailing = chars.collect();
                break;
            } else {
                result.push(c);
            }
        }

        if !closed {
            return Err(format!("line {}: unterminated double-quoted value", lineno));
        }
        let trailing = trailing.trim();
        if !trailing.is_empty() && !trailing.starts_with('#') {
            return Err(format!(
                "line {}: unexpected trailing data after quoted value",
                lineno
            ));
        }
        Ok(result)
    } else {
        let mut cut = None;
        for (i, c) in input.char_indices() {
            if c == '#' && i > 0 {
                if input[..i].chars().last().map(char::is_whitespace).unwrap_or(false) {
                    cut = Some(i);
                    break;
                }
            }
        }
        let value = match cut {
            Some(i) => input[..i].trim(),
            None => input.trim(),
        };
        Ok(value.to_string())
    }
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
        writeln!(f, r#"export BAZ="qux""#).unwrap();
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
        writeln!(f, "export{}BAZ=qux", char::from(9)).unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "bar");
        assert_eq!(m.get("BAZ").unwrap(), "qux");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn strips_byte_order_mark() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-bom.env");
        let mut f = fs::File::create(&p).unwrap();
        write!(f, "{}", char::from_u32(0xFEFF).unwrap()).unwrap();
        writeln!(f, "FOO=bar").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "bar");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn export_prefix_requires_word_boundary() {
        let dir = std::env::temp_dir();
        let p = dir.join("envguard-export-boundary.env");
        let mut f = fs::File::create(&p).unwrap();
        writeln!(f, "exportFOO=bar").unwrap();
        writeln!(f, "export BAR=baz").unwrap();
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
        let mut f = fs::File::create(&p).unwrap();
        writeln!(f, r#"FOO="hello world""#).unwrap();
        writeln!(f, "BAR='x y'").unwrap();
        let m = load_dotenv(&p).unwrap();
        assert_eq!(m.get("FOO").unwrap(), "hello world");
        assert_eq!(m.get("BAR").unwrap(), "x y");
        let _ = fs::remove_file(&p);
    }
}
