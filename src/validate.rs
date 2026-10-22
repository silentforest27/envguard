use crate::schema::{Schema, VarType};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub var: String,
    pub message: String,
}

pub fn check_type(ty: &VarType, value: &str) -> Result<(), String> {
    match ty {
        VarType::String => Ok(()),
        VarType::Int => {
            let chars_ok = !value.is_empty()
                && value.bytes().iter().enumerate().all(|(i, &b)| {
                    b.is_ascii_digit() || (i == 0 && (b == b'+' || b == b'-'))
                });
            if chars_ok && value.parse::<i64>().is_ok() {
                Ok(())
            } else {
                Err(format!("invalid value {:?}: expected int", value))
            }
        }
        VarType::Bool => match value.to_lowercase().as_str() {
            "true" | "false" | "1" | "0" | "yes" | "no" => Ok(()),
            _ => Err(format!("invalid value {:?}: expected bool", value)),
        },
        VarType::Url => {
            let (scheme, rest) = value
                .split_once("://")
                .ok_or_else(|| format!("invalid value {:?}: expected url", value))?;
            let scheme_ok = !scheme.is_empty()
                && scheme
                    .chars()
                    .next()
                    .map(char::is_ascii_alphabetic)
                    .unwrap_or(false)
                && scheme.chars().all(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.' | '_')
                });
            if !scheme_ok
                || rest.is_empty()
                || value.contains(char::is_whitespace)
            {
                return Err(format!("invalid value {:?}: expected url", value));
            }
            let authority_end = rest
                .find(|c| c == '/' || c == '?' || c == '#')
                .unwrap_or(rest.len());
            let authority = &rest[..authority_end];
            if authority.is_empty() {
                return Err(format!("invalid value {:?}: expected url", value));
            }

            let host_part = match authority.rfind('@') {
                Some(0) => {
                    return Err(format!("invalid value {:?}: expected url", value))
                }
                Some(i) => &authority[i + 1..],
                None => authority,
            };
            if host_part.is_empty() {
                return Err(format!("invalid value {:?}: expected url", value));
            }

            let (host, port, ipv6) = if let Some(after_bracket) = host_part.strip_prefix('[') {
                let close = match after_bracket.find(']') {
                    Some(i) if i > 0 => i,
                    _ => return Err(format!("invalid value {:?}: expected url", value)),
                };
                let host = &after_bracket[..close];
                let remainder = &after_bracket[close + 1..];
                let port = if remainder.is_empty() {
                    ""
                } else if let Some(stripped) = remainder.strip_prefix(':') {
                    if stripped.is_empty() {
                        return Err(format!("invalid value {:?}: expected url", value));
                    }
                    stripped
                } else {
                    return Err(format!("invalid value {:?}: expected url", value));
                };
                if !host.chars().all(|c| c.is_ascii_hexdigit() || c == ':')
                    || !host.contains(':')
                {
                    return Err(format!("invalid value {:?}: expected url", value));
                }
                (host, port, true)
            } else if let Some(colon) = host_part.rfind(':') {
                let host = &host_part[..colon];
                let port = &host_part[colon + 1..];
                if host.is_empty() || host.contains(':') || port.is_empty() {
                    return Err(format!("invalid value {:?}: expected url", value));
                }
                (host, port, false)
            } else {
                (host_part, "", false)
            };

            if host.is_empty() || !valid_host(host, ipv6) {
                return Err(format!("invalid value {:?}: expected url", value));
            }
            if !port.is_empty() {
                if !port.bytes().all(u8::is_ascii_digit) {
                    return Err(format!("invalid value {:?}: expected url", value));
                }
                let n: u32 = port
                    .parse()
                    .map_err(|_| format!("invalid value {:?}: expected url", value))?;
                if !(1..=65535).contains(&n) {
                    return Err(format!("invalid value {:?}: expected url", value));
                }
            }
            Ok(())
        }
        VarType::Port => {
            if value.is_empty() || !value.bytes().all(u8::is_ascii_digit) {
                return Err(format!("invalid value {:?}: expected port (1-65535)", value));
            }
            let n: u32 = value
                .parse()
                .map_err(|_| format!("invalid value {:?}: expected port (1-65535)", value))?;
            if (1..=65535).contains(&n) {
                Ok(())
            } else {
                Err(format!("invalid value {:?}: expected port (1-65535)", value))
            }
        }
    }
}

fn valid_host(host: &str, ipv6: bool) -> bool {
    if ipv6 {
        host.parse::<std::net::Ipv6Addr>().is_ok()
    } else if host.split('.').count() == 4
        && host.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    {
        host.parse::<std::net::Ipv4Addr>().is_ok()
    } else {
        if host.is_empty() {
            return false;
        }
        let host = host.strip_suffix('.').unwrap_or(host);
        if host.is_empty() {
            return false;
        }
        for label in host.split('.') {
            if label.is_empty()
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return false;
            }
        }
        true
    }
}

pub fn validate(schema: &Schema, env: &BTreeMap<String, String>) -> Vec<Issue> {
    let mut issues = Vec::new();
    for spec in &schema.vars {
        let val = env.get(&spec.name).cloned().or_else(|| spec.default.clone());
        match val {
            None => {
                if spec.required {
                    issues.push(Issue {
                        var: spec.name.clone(),
                        message: "missing required variable".to_string(),
                    });
                }
            }
            Some(v) => {
                if let Err(e) = check_type(&spec.ty, &v) {
                    issues.push(Issue {
                        var: spec.name.clone(),
                        message: e,
                    });
                }
            }
        }
    }
    issues
}

pub fn resolved_values(schema: &Schema, env: &BTreeMap<String, String>) -> Vec<(String, Option<String>)> {
    schema
        .vars
        .iter()
        .map(|s| {
            let v = env.get(&s.name).cloned().or_else(|| s.default.clone());
            (s.name.clone(), v)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;

    #[test]
    fn missing_required() {
        let s = Schema::parse("API_KEY required string").unwrap();
        let env = BTreeMap::new();
        let issues = validate(&s, &env);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].var, "API_KEY");
    }

    #[test]
    fn default_used() {
        let s = Schema::parse("PORT optional port 8080").unwrap();
        let env = BTreeMap::new();
        let issues = validate(&s, &env);
        assert!(issues.is_empty());
    }

    #[test]
    fn type_mismatch() {
        let s = Schema::parse("PORT optional port 8080").unwrap();
        let mut env = BTreeMap::new();
        env.insert("PORT".into(), "abc".into());
        let issues = validate(&s, &env);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn bool_and_url_ok() {
        let s = Schema::parse(r#"D required url
B required bool"#).unwrap();
        let mut env = BTreeMap::new();
        env.insert("D".into(), "https://x/y".into());
        env.insert("B".into(), "true".into());
        assert!(validate(&s, &env).is_empty());
    }

    #[test]
    fn url_requires_scheme_and_nonempty_value() {
        let s = Schema::parse("U required url").unwrap();
        for value in [
            "://host",
            "http://",
            "not-a-url",
            "http://example.com/path with space",
            "http://:8080",
        ] {
            let mut env = BTreeMap::new();
            env.insert("U".into(), value.into());
            assert!(!validate(&s, &env).is_empty(), "{} should be invalid", value);
        }
        let mut env = BTreeMap::new();
        env.insert("U".into(), "https://example.com/path".into());
        assert!(validate(&s, &env).is_empty());
    }

    #[test]
    fn url_rejects_malformed_components() {
        let s = Schema::parse("U required url").unwrap();
        for value in [
            "http://example.com:",
            "http://example.com:abc",
            "http://example.com:0",
            "http://example.com:70000",
            "http://[::1",
            "http://[]",
            "http://example.com::80",
            "http://@example.com",
            "http://example.com@",
        ] {
            let mut env = BTreeMap::new();
            env.insert("U".into(), value.into());
            assert!(!validate(&s, &env).is_empty(), "{} should be invalid", value);
        }
        for value in [
            "http://example.com",
            "https://example.com/path",
            "http://[::1]:8080/path",
            "http://[::1]",
            "http://localhost:3000",
            "http://127.0.0.1:8080",
        ] {
            let mut env = BTreeMap::new();
            env.insert("U".into(), value.into());
            assert!(validate(&s, &env).is_empty(), "{} should be valid", value);
        }
    }

    #[test]
    fn url_rejects_malformed_hosts() {
        let s = Schema::parse("U required url").unwrap();
        for value in [
            "http://example..com",
            "http://-example.com",
            "http://example-.com",
            "http://.example.com",
            "http://999.999.999.999",
            "http:///path",
        ] {
            let mut env = BTreeMap::new();
            env.insert("U".into(), value.into());
            assert!(!validate(&s, &env).is_empty(), "{} should be invalid", value);
        }
        for value in [
            "http://example.com.",
            "http://example.com",
            "http://a_b.example.com",
            "http://[::1]",
        ] {
            let mut env = BTreeMap::new();
            env.insert("U".into(), value.into());
            assert!(validate(&s, &env).is_empty(), "{} should be valid", value);
        }
    }
}
