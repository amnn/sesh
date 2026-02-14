use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(name) = args.next() else {
        anyhow::bail!("usage: cargo run -p sesh-integration-tests --bin new-test -- <name>");
    };
    if args.next().is_some() {
        anyhow::bail!("expected exactly one test name argument");
    }

    let file_name = format!("{}.md", name.replace(' ', "-"));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cases")
        .join(file_name);

    if path.exists() {
        anyhow::bail!("test case already exists: {}", path.display());
    }

    let template = r#"# New integration test

> [!INFO]
> **Directive quick reference**
>
> | Directive | Description |
> | --- | --- |
> | `:s` / `:sh <cmd...>` | Run host command (Rust `Command` + shlex args) |
> | `:t` / `:tmux <args...>` | Run tmux command on test socket |
> | `:p` / `:pane <target>` | Change current tmux pane target |
> | `:k` / `:keys <tokens...>` | Send key tokens and/or quoted text |
> | `:snap [dregexdrepld ...]` | Capture current pane with optional replacements |

## Setup

:t new-session -d -s fixture -x 120 -y 40 "sleep 3600"

## Exercise

:k down up

## Snapshot

:snap /zz-sesh-ui-runner/<RUNNER>/ /[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/*/
"#;

    fs::write(&path, template)?;
    println!("Created {}", path.display());
    Ok(())
}
