use envguard::{load_dotenv, validate, resolved_values};
use std::collections::BTreeMap;
use std::process;

fn print_help() {
    println!("envguard - validate environment variables against a schema");
    println!();
    println!("Usage: envguard [--schema FILE] [--env FILE] [--print] [--json]");
    println!();
    println!("  --schema FILE   schema file (default .env.schema)");
    println!("  --env FILE      env file (default .env; missing file is ok)");
    println!("  --print         print resolved values");
    println!("  --json          print machine-readable JSON result");
    println!("  --version       print version and exit");
    println!("  -h, --help      show this help");
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut schema_path = String::from(".env.schema");
    let mut env_path = String::from(".env");
    let mut print = false;
    let mut json = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--schema" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("envguard: --schema requires a value");
                    process::exit(2);
                }
                schema_path = args[i].clone();
            }
            arg if arg.starts_with("--schema=") => {
                schema_path = arg["--schema=".len()..].to_string();
            }
            "--env" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("envguard: --env requires a value");
                    process::exit(2);
                }
                env_path = args[i].clone();
            }
            arg if arg.starts_with("--env=") => {
                env_path = arg["--env=".len()..].to_string();
            }
            "--print" => print = true,
            "--json" => json = true,
            "--version" => {
                println!("envguard {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            other => {
                eprintln!("envguard: unknown argument '{}'", other);
                process::exit(2);
            }
        }
        i += 1;
    }

    let schema = match envguard::Schema::from_file(std::path::Path::new(&schema_path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("envguard: {}", e);
            process::exit(2);
        }
    };

    let mut env: BTreeMap<String, String> = match load_dotenv(std::path::Path::new(&env_path)) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("envguard: {}", e);
            process::exit(2);
        }
    };
    for (k, v) in std::env::vars() {
        env.insert(k, v);
    }

    let issues = validate(&schema, &env);
    if json {
        let mut values = String::from("{");
        let rv = resolved_values(&schema, &env);
        for (idx, (name, val)) in rv.iter().enumerate() {
            if idx > 0 {
                values.push(',');
            }
            values.push('"');
            values.push_str(&json_escape(name));
            values.push_str("\":");
            match val {
                Some(v) => {
                    values.push('"');
                    values.push_str(&json_escape(v));
                    values.push('"');
                }
                None => values.push_str("null"),
            }
        }
        values.push('}');

        let mut issues_json = String::from("[");
        for (idx, issue) in issues.iter().enumerate() {
            if idx > 0 {
                issues_json.push(',');
            }
            issues_json.push_str(&format!(
                "{{\"var\":\"{}\",\"message\":\"{}\"}}",
                json_escape(&issue.var),
                json_escape(&issue.message)
            ));
        }
        issues_json.push(']');

        println!(
            "{{\"ok\":{},\"issues\":{},\"values\":{}}}",
            issues.is_empty(),
            issues_json,
            values
        );
    } else if print {
        for (name, val) in resolved_values(&schema, &env) {
            match val {
                Some(v) => println!("{}={}", name, v),
                None => println!("{}=<unset>", name),
            }
        }
    }
    if issues.is_empty() {
        if !print && !json {
            println!("envguard: all variables valid");
        }
        process::exit(0);
    } else {
        if !json {
            for issue in &issues {
                eprintln!("{}: {}", issue.var, issue.message);
            }
        }
        process::exit(1);
    }
}
