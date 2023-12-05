use std::error::Error as StdError;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::prelude::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

type Error = Box<dyn StdError>;
type Result<T> = core::result::Result<T, Error>;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.len() != 1 {
        help()?;
    }

    let project_root = &project_root();
    match args[0].as_str() {
        "install-pre-commit-hook" => {
            install_pre_commit_hook(project_root, "ci")?;
        }

        "ci-install" => run_ci_install()?,

        "ci" => run_ci(project_root)?,

        _ => help()?,
    }

    Ok(())
}

fn run_ci(project_root: &Path) -> Result<()> {
    if rustversion::cfg!(nightly) {
        run(Command::new("cargo")
            .args(["miri", "test"])
            .current_dir(project_root))?;

        run(Command::new("cargo")
            .args(["fuzz", "build"])
            .current_dir(project_root))?;

        run(Command::new("cargo")
            .args(["rustdoc", "--", "--deny", "warnings"])
            .current_dir(project_root))?;

        let manifests = ["Cargo.toml", "fuzz/Cargo.toml", "thumbv7em/Cargo.toml"];

        for manifest in manifests {
            run(Command::new("cargo")
                .args(["fmt", "--check", "--all", "--manifest-path", manifest])
                .current_dir(project_root))?;
        }
    } else {
        run(Command::new("cargo").arg("test").current_dir(project_root))?;

        run(Command::new("cargo")
            .args(["build", "--bin", "no-panics"])
            .current_dir(project_root.join("thumbv7em")))?;

        let subdirs = [".", "thumbv7em"];

        for subdir in subdirs {
            run(Command::new("cargo")
                .args(["clippy", "--all-targets", "--", "--deny", "warnings"])
                .current_dir(project_root.join(subdir)))?;
        }
    }

    Ok(())
}

fn run_ci_install() -> Result<()> {
    run(Command::new("rustup").args(["target", "add", "thumbv7em-none-eabi"]))?;

    let components: &[_] = if rustversion::cfg!(nightly) {
        &["miri", "rustfmt", "rust-src"]
    } else {
        &["clippy"]
    };
    run(Command::new("rustup")
        .args(["component", "add"])
        .args(components))?;

    Ok(())
}

fn install_pre_commit_hook(project_root: &Path, subcommand: &str) -> Result<()> {
    let git_dir = project_root.join(".git");

    if !git_dir.exists() {
        return Err("not in a git repository".into());
    }

    let hooks_dir = git_dir.join("hooks");
    let _ = fs::create_dir(&hooks_dir);

    let pre_commit_file = hooks_dir.join("pre-commit");
    let _ = fs::remove_file(&pre_commit_file);

    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .mode(0o755)
        .open(pre_commit_file)?;
    writeln!(
        writer,
        "#!/usr/bin/env bash
set -euo pipefail

cargo +stable xtask {subcommand}
cargo +nightly xtask {subcommand}"
    )?;

    Ok(())
}

fn run(command: &mut Command) -> Result<()> {
    eprintln!("{command:?}");

    if command.status()?.success() {
        Ok(())
    } else {
        Err(format!("'{command:?}' failed").into())
    }
}

fn help() -> Result<()> {
    eprintln!("xtask subcommand must be one of: install-pre-commit-hook, ci");
    Err("incorrect command-line usage".into())
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
