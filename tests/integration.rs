use std::fs;
use std::process::Command;

fn tmpdir(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("envguard-it-{}", name));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn missing_required_fails() {
    let dir = tmpdir("missing");
    fs::write(dir.join(".env.schema"), "API_KEY required string\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_envguard"))
        .arg("--schema")
        .arg(dir.join(".env.schema"))
        .arg("--env")
        .arg(dir.join(".env"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("API_KEY"));
}

#[test]
fn valid_with_defaults_and_print() {
    let dir = tmpdir("valid");
    fs::write(dir.join(".env.schema"), "PORT optional port 8080\nDB required url\n").unwrap();
    fs::write(dir.join(".env"), "DB=https://example.com/db\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_envguard"))
        .arg("--schema")
        .arg(dir.join(".env.schema"))
        .arg("--env")
        .arg(dir.join(".env"))
        .arg("--print")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("PORT=8080"));
    assert!(stdout.contains("DB=https://example.com/db"));
}

#[test]
fn invalid_type_fails() {
    let dir = tmpdir("invalid");
    fs::write(dir.join(".env.schema"), "PORT optional port 8080\n").unwrap();
    fs::write(dir.join(".env"), "PORT=notanumber\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_envguard"))
        .arg("--schema")
        .arg(dir.join(".env.schema"))
        .arg("--env")
        .arg(dir.join(".env"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("PORT"));
}
