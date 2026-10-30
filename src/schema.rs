use crate::validate::check_type;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarType {
    String,
    Int,
    Bool,
    Url,
    Port,
}

#[derive(Debug, Clone)]
pub struct VarSpec {
    pub name: String,
    pub required: bool,
    pub ty: VarType,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub vars: Vec<VarSpec>,
}

impl Schema {
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut vars = Vec::new();
        let mut seen = BTreeSet::new();
        for (lineno, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                return Err(format!(
                    "line {}: expected at least 3 fields (name required|optional type [default])",
                    lineno + 1
                ));
            }
            let name = parts[0].to_string();
            if !is_valid_name(&name) {
                return Err(format!(
                    "line {}: invalid variable name '{}'",
                    lineno + 1,
                    name
                ));
            }
            if !seen.insert(name.clone()) {
                return Err(format!("line {}: duplicate variable '{}'", lineno + 1, name));
            }
            let required = match parts[1] {
                "required" => true,
                "optional" => false,
                other => {
                    return Err(format!(
                        "line {}: expected 'required' or 'optional', got '{}'",
                        lineno + 1,
                        other
                    ))
                }
            };
            let ty = match parts[2] {
                "string" => VarType::String,
                "int" => VarType::Int,
                "bool" => VarType::Bool,
                "url" => VarType::Url,
                "port" => VarType::Port,
                other => {
                    return Err(format!("line {}: unknown type '{}'", lineno + 1, other))
                }
            };
            let default = if parts.len() >= 4 {
                if required {
                    return Err(format!(
                        "line {}: default value not allowed for required variable '{}'",
                        lineno + 1,
                        name
                    ));
                }
                let default_value = parts[3..].join(" ");
                if let Err(e) = check_type(&ty, &default_value) {
                    return Err(format!(
                        "line {}: invalid default value for '{}': {}",
                        lineno + 1,
                        name,
                        e
                    ));
                }
                Some(default_value)
            } else {
                None
            };
            vars.push(VarSpec {
                name,
                required,
                ty,
                default,
            });
        }
        Ok(Schema { vars })
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        Self::parse(&text)
    }
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let s = Schema::parse("DATABASE_URL required url\nPORT optional port 8080\n").unwrap();
        assert_eq!(s.vars.len(), 2);
        assert!(s.vars[0].required);
        assert_eq!(s.vars[1].default.as_deref(), Some("8080"));
    }

    #[test]
    fn ignores_comments_and_blanks() {
        let s = Schema::parse("# c\n\nFOO optional string\n").unwrap();
        assert_eq!(s.vars.len(), 1);
    }

    #[test]
    fn duplicate_errors() {
        let r = Schema::parse("FOO optional string\nFOO optional string\n");
        assert!(r.is_err());
    }

    #[test]
    fn unknown_type_errors() {
        let r = Schema::parse("FOO optional float\n");
        assert!(r.is_err());
    }

    #[test]
    fn too_few_fields_errors() {
        let r = Schema::parse("FOO optional\n");
        assert!(r.is_err());
    }

    #[test]
    fn required_default_errors() {
        let r = Schema::parse("FOO required string default\n");
        assert!(r.is_err());
    }

    #[test]
    fn strips_byte_order_mark() {
        let s = Schema::parse("\u{feff}FOO optional string\n").unwrap();
        assert_eq!(s.vars.len(), 1);
        assert_eq!(s.vars[0].name, "FOO");
    }

    #[test]
    fn invalid_name_errors() {
        assert!(Schema::parse("1FOO optional string\n").is_err());
        assert!(Schema::parse("FOO-BAR optional string\n").is_err());
    }

    #[test]
    fn invalid_type_default_errors() {
        assert!(Schema::parse("PORT optional port not-a-port\n").is_err());
        assert!(Schema::parse("FLAG optional bool definitely\n").is_err());
        assert!(Schema::parse("N optional int x\n").is_err());
        assert!(Schema::parse("URL optional url example.com\n").is_err());
    }

    #[test]
    fn valid_type_default_accepted() {
        let s = Schema::parse(
            "PORT optional port 8080\nFLAG optional bool false\nN optional int 42\nURL optional url https://example.com/path\n",
        )
        .unwrap();
        assert_eq!(s.vars.len(), 4);
    }
}
