# envguard

`envguard` is a small, dependency-free Rust CLI and library that validates
environment variables against a declarative schema before your application
starts. It catches missing required variables and type mismatches early,
loads values from a `.env` file, and applies defaults.

## Features

- Required and optional variables
- Type checking: `string`, `int`, `bool`, `url`, `port`
- Default values for optional variables
- Loads `.env` files (with optional `export` prefix and quoted values)
- Process environment overrides `.env` values
- Clear diagnostics and non-zero exit code on failure
- Usable as a library (`envguard` crate) or as a CLI

## Schema format

One variable per line, whitespace-separated fields:

```
# name required|optional type [default]
DATABASE_URL required url
PORT optional port 8080
DEBUG optional bool false
API_KEY required string
MAX_RETRIES optional int 3
```

Lines starting with `#` and blank lines are ignored. Duplicate variable names
are rejected.

## Usage

Create a schema file (default `.env.schema`) and optionally a `.env` file:

```
# .env.schema
DATABASE_URL required url
PORT optional port 8080
```

```
# .env
DATABASE_URL=https://example.com/db
```

Run:

```
envguard
```

On success:

```
envguard: all variables valid
```

On failure (exit code 1):

```
PORT: invalid value "abc": expected port (1-65535)
API_KEY: missing required variable
```

### Options

```
envguard [--schema FILE] [--env FILE] [--print]
```

- `--schema FILE`  schema file (default `.env.schema`)
- `--env FILE`     env file (default `.env`; missing file is not an error)
- `--print`        print resolved values (name=value or name=<unset>)
- `-h, --help`     show help

### Library usage

```rust
use envguard::{Schema, validate, load_dotenv};
use std::collections::BTreeMap;

let schema = Schema::parse("PORT optional port 8080\n").unwrap();
let mut env = BTreeMap::new();
env.insert("PORT".into(), "9090".into());
let issues = validate(&schema, &env);
assert!(issues.is_empty());
```

## Build and test

```
cargo build
 cargo test
```

No external dependencies are required.
