//! build_dist.rs: Standalone root build orchestrator for Project32 duct.
//!
//! Can be invoked via:
//!     cargo run --bin build_dist -- --all
//!     cargo run --bin build_dist -- --only gl
//! Or directly delegates to cargo xtask.

use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--package").arg("xtask").arg("--").arg("build");
    if args.is_empty() {
        cmd.arg("--all");
    } else {
        for arg in &args {
            cmd.arg(arg);
        }
    }

    println!("[build_dist] Delegating to xtask build orchestrator...");
    let status = cmd.status().expect("Failed to execute cargo xtask");
    std::process::exit(status.code().unwrap_or(0));
}
