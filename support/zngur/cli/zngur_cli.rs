// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! A minimal zngur command line, for build rules: generates the bindings of a `.zng` file, or
//! zngur's runtime header (`zngur.h`), with the `zngur` crate.
//!
//! Unlike zngur's own CLI, `generate` always writes the C++ file (empty if the bindings don't
//! need one), since build systems like Bazel need every declared output.

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use zngur::{Zngur, ZngurHdr};

#[derive(Parser)]
#[command(about = "Generates zngur bindings")]
struct Cmdline {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generates the bindings of a `.zng` file.
    Generate {
        /// The `.zng` file.
        zng: PathBuf,
        /// The generated C++ header.
        #[arg(long)]
        h_file: PathBuf,
        /// The generated Rust file.
        #[arg(long)]
        rs_file: PathBuf,
        /// The generated C++ file, written even if empty.
        #[arg(long)]
        cpp_file: PathBuf,
        /// The name of the crate that the Rust file is part of (`#crate` by default).
        #[arg(long)]
        crate_name: Option<String>,
        /// Included in the symbols of the generated functions, to keep them unique.
        #[arg(long)]
        mangling_base: Option<String>,
        /// A rustc, to get the Rust `cfg`s of the target (`rustc --print cfg`), for the
        /// conditional compilation in `.zng` files (`#if cfg!(...)`).
        #[arg(long)]
        rustc: Option<PathBuf>,
        /// The target for `--rustc`.
        #[arg(long, requires = "rustc")]
        target: Option<String>,
    },
    /// Writes zngur's runtime header, which every generated header includes as `zngur.h`.
    MakeZngHeader {
        /// The output file.
        path: PathBuf,
    },
}

/// The `cfg`s of a target, from `rustc --print cfg`: e.g. `target_pointer_width="64"` is
/// `("target_pointer_width", ["64"])`, and `unix` is `("unix", [""])`.
fn rustc_cfg(rustc: &Path, target: Option<&str>) -> Result<HashMap<String, Vec<String>>, String> {
    let mut command = std::process::Command::new(rustc);
    if let Some(target) = target {
        command.args(["--target", target]);
    }
    let output = command
        .args(["--print", "cfg"])
        .output()
        .map_err(|err| format!("Failed to run {}: {err}", rustc.display()))?;
    if !output.status.success() {
        return Err(format!(
            "`{} --print cfg` failed:\n{}",
            rustc.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut cfg: HashMap<String, Vec<String>> = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
        cfg.entry(key.to_owned()).or_default().push(value.to_owned());
    }
    Ok(cfg)
}

fn main() -> Result<(), String> {
    match Cmdline::parse().command {
        Command::Generate {
            zng,
            h_file,
            rs_file,
            cpp_file,
            crate_name,
            mangling_base,
            rustc,
            target,
        } => {
            // zngur only writes the C++ file if the bindings need one.
            std::fs::write(&cpp_file, "")
                .map_err(|err| format!("Failed to write {}: {err}", cpp_file.display()))?;
            let mut zngur = Zngur::from_zng_file(zng)
                .with_h_file(h_file)
                .with_rs_file(rs_file)
                .with_cpp_file(cpp_file)
                .with_warning_sink(|warning| eprintln!("{warning}"));
            if let Some(crate_name) = &crate_name {
                zngur = zngur.with_crate_name(crate_name);
            }
            if let Some(mangling_base) = &mangling_base {
                zngur = zngur.with_mangling_base(mangling_base);
            }
            if let Some(rustc) = &rustc {
                zngur = zngur.with_rust_in_memory_cfg(rustc_cfg(rustc, target.as_deref())?);
            }
            zngur.try_generate()
        }
        Command::MakeZngHeader { path } => ZngurHdr::new().with_zng_header(path).generate(),
    }
}
