use envguard::{load_dotenv, validate, resolved_values};
use std::collections::BTreeMap;
use std::process;

fn print_help() {
    println!("envguard - validate environment variables against a schema");
    println!();
    println!("Usage: envguard [--schema FILE] [--env FILE] [--print]");
    println!();
    println!("  --schema FILE   schema file (default .env.schema)");
    println!("  --env FILE      env file (default .env; missing file is ok)");
    println!("  --print         print resolved values");
    println!("  -h, --help      show this help");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut schema_path = String::from(".env.schema");
    let mut env_path = String::from(".env");
    let mut print = false;
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
            "--env" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("envguard: --env requires a value");
                    process::exit(2);
                }
                env_path = args[i].clone();
            }
            "--print" => print = true,
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
    if print {
        for (name, val) in resolved_values(&schema, &env) {
            match val {
                Some(v) => println!("{}={}", name, v),
                None => println!("{}=<unset>", name),
            }
        }
    }
    if issues.is_empty() {
        if !print {
            println!("envguard: all variables valid");
        }
        process::exit(0);
    } else {
        for issue in &issues {
            eprintln!("{}: {}", issue.var, issue.message);
        }
        process::exit(1);
    }
}
