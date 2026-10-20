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
        VarType::Int => value
            .parse::<i64>()
            .map(|_| ())
            .map_err(|_| format!("invalid value {:?}: expected int", value)),
        VarType::Bool => match value.to_lowercase().as_str() {
            "true" | "false" | "1" | "0" | "yes" | "no" => Ok(()),
            _ => Err(format!("invalid value {:?}: expected bool", value)),
        },
        VarType::Url => {
            if let Some((scheme, rest)) = value.split_once("://") {
                if !scheme.is_empty()
                    && !rest.is_empty()
                    && !value.contains(char::is_whitespace)
                {
                    Ok(())
                } else {
                    Err(format!("invalid value {:?}: expected url", value))
                }
            } else {
                Err(format!("invalid value {:?}: expected url", value))
            }
        }
        VarType::Port => {
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
        let s = Schema::parse("API_KEY required string\n").unwrap();
        let env = BTreeMap::new();
        let issues = validate(&s, &env);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].var, "API_KEY");
    }

    #[test]
    fn default_used() {
        let s = Schema::parse("PORT optional port 8080\n").unwrap();
        let env = BTreeMap::new();
        let issues = validate(&s, &env);
        assert!(issues.is_empty());
    }

    #[test]
    fn type_mismatch() {
        let s = Schema::parse("PORT optional port 8080\n").unwrap();
        let mut env = BTreeMap::new();
        env.insert("PORT".into(), "abc".into());
        let issues = validate(&s, &env);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn bool_and_url_ok() {
        let s = Schema::parse("D required url\nB required bool\n").unwrap();
        let mut env = BTreeMap::new();
        env.insert("D".into(), "https://x/y".into());
        env.insert("B".into(), "true".into());
        assert!(validate(&s, &env).is_empty());
    }

    #[test]
    fn url_requires_scheme_and_nonempty_value() {
        let s = Schema::parse("U required url\n").unwrap();
        for value in ["://host", "http://", "not-a-url", "http://example.com/path with space"] {
            let mut env = BTreeMap::new();
            env.insert("U".into(), value.into());
            assert!(!validate(&s, &env).is_empty(), "{} should be invalid", value);
        }
        let mut env = BTreeMap::new();
        env.insert("U".into(), "https://example.com/path".into());
        assert!(validate(&s, &env).is_empty());
    }
}
