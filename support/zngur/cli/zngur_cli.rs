// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! A minimal zngur command line, for build rules: generates the bindings of a `.zng` file, or
//! zngur's runtime header (`zngur.h`), with the `zngur` crate.
//!
//! Unlike zngur's own CLI, `generate` always writes the C++ file (empty if the bindings don't
//! need one), since build systems like Bazel need every declared output.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
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
    },
    /// Writes zngur's runtime header, which every generated header includes as `zngur.h`.
    MakeZngHeader {
        /// The output file.
        path: PathBuf,
    },
}

fn main() -> Result<(), String> {
    match Cmdline::parse().command {
        Command::Generate { zng, h_file, rs_file, cpp_file, crate_name, mangling_base } => {
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
            zngur.try_generate()
        }
        Command::MakeZngHeader { path } => ZngurHdr::new().with_zng_header(path).generate(),
    }
}
