//! xtask: Build orchestration and artifact distribution tool for Project32 duct.
//!
//! Handles selective compilation of graphics pipeline modules, artifact renaming,
//! and distribution into `./renderer/` and `target/<profile>/renderer/`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Build,
    Clean,
    Help,
}

#[derive(Debug)]
struct BuildConfig {
    release: bool,
    build_gl: bool,
    build_dx11: bool,
    build_dx12: bool,
    build_vk: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            release: false,
            build_gl: true,
            build_dx11: true,
            build_dx12: true,
            build_vk: true,
        }
    }
}

fn print_help() {
    println!(
        r#"
Project32 Duct Build Orchestrator (xtask)

USAGE:
    cargo xtask build [OPTIONS]
    cargo xtask clean
    cargo xtask help

OPTIONS:
    --all            Build all graphics backends (OpenGL, DirectX 11, DirectX 12, Vulkan) [Default]
    --only <API>     Build only specific backend: 'gl', 'dx11', 'dx12', 'vk' (comma-separated allowed)
    --release        Build artifacts with release optimizations
    -h, --help       Show this help message

EXAMPLES:
    cargo xtask build --all
    cargo xtask build --only gl
    cargo xtask build --only dx11,dx12,vk --release
    cargo xtask clean
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        return;
    }

    let mut action = Action::Help;
    let mut config = BuildConfig::default();
    let mut explicit_only = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "build" => action = Action::Build,
            "clean" => action = Action::Clean,
            "help" | "-h" | "--help" => action = Action::Help,
            "--release" => config.release = true,
            "--all" => {
                config.build_gl = true;
                config.build_dx11 = true;
                config.build_dx12 = true;
                config.build_vk = true;
                explicit_only = false;
            }
            "--only" => {
                if i + 1 < args.len() {
                    i += 1;
                    let targets = &args[i];
                    if !explicit_only {
                        config.build_gl = false;
                        config.build_dx11 = false;
                        config.build_dx12 = false;
                        config.build_vk = false;
                        explicit_only = true;
                    }
                    for target in targets.split(',') {
                        match target.trim().to_lowercase().as_str() {
                            "gl" | "opengl" => config.build_gl = true,
                            "dx11" | "d3d11" => config.build_dx11 = true,
                            "dx12" | "d3d12" => config.build_dx12 = true,
                            "vk" | "vulkan" => config.build_vk = true,
                            other => {
                                eprintln!("[xtask][ERROR] Unknown backend target '{}'", other);
                                std::process::exit(1);
                            }
                        }
                    }
                } else {
                    eprintln!("[xtask][ERROR] Flag '--only' requires an argument (e.g. --only gl)");
                    std::process::exit(1);
                }
            }
            unknown => {
                eprintln!("[xtask][ERROR] Unknown argument '{}'", unknown);
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    match action {
        Action::Help => print_help(),
        Action::Clean => execute_clean(),
        Action::Build => execute_build(&config),
    }
}

fn execute_clean() {
    println!("[xtask] Cleaning renderer directories...");
    let root = workspace_root();

    let local_renderer = root.join("renderer");
    if local_renderer.exists() {
        let _ = fs::remove_dir_all(&local_renderer);
        let _ = fs::create_dir_all(&local_renderer);
        println!("[xtask] Cleared {}", local_renderer.display());
    }

    let debug_renderer = root.join("target").join("debug").join("renderer");
    if debug_renderer.exists() {
        let _ = fs::remove_dir_all(&debug_renderer);
        println!("[xtask] Cleared {}", debug_renderer.display());
    }

    let release_renderer = root.join("target").join("release").join("renderer");
    if release_renderer.exists() {
        let _ = fs::remove_dir_all(&release_renderer);
        println!("[xtask] Cleared {}", release_renderer.display());
    }

    println!("[xtask] Clean complete.");
}

fn execute_build(config: &BuildConfig) {
    let profile_name = if config.release { "release" } else { "debug" };
    println!(
        "[xtask] === Building Project32 Duct Subsystem [Profile: {}] ===",
        profile_name
    );

    let mut packages = vec!["duct"];
    if config.build_gl {
        packages.push("glduct");
    }
    if config.build_dx11 {
        packages.push("dx11duct");
    }
    if config.build_dx12 {
        packages.push("dx12duct");
    }
    if config.build_vk {
        packages.push("vkduct");
    }

    println!("[xtask] Target packages to compile: {:?}", packages);

    let mut cargo_args = vec!["build"];
    for p in &packages {
        cargo_args.push("-p");
        cargo_args.push(p);
    }
    if config.release {
        cargo_args.push("--release");
    }

    println!("[xtask] Running: cargo {}", cargo_args.join(" "));
    let status = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .expect("Failed to execute cargo build");

    if !status.success() {
        eprintln!("[xtask][ERROR] Cargo build command failed.");
        std::process::exit(status.code().unwrap_or(1));
    }

    deploy_artifacts(config);
}

fn deploy_artifacts(config: &BuildConfig) {
    let root = workspace_root();
    let profile = if config.release { "release" } else { "debug" };
    let target_dir = root.join("target").join(profile);
    let target_renderer = target_dir.join("renderer");
    let local_renderer = root.join("renderer");

    fs::create_dir_all(&target_renderer).expect("Failed to create target renderer dir");
    fs::create_dir_all(&local_renderer).expect("Failed to create local renderer dir");

    let (prefix, ext) = get_platform_dll_spec();

    // 1. Copy duct.dll to target/<profile>/ (if built as cdylib)
    let raw_duct_name = format!("{}duct.{}", prefix, ext);
    let raw_duct_path = target_dir.join(&raw_duct_name);
    let out_duct_name = format!("duct.{}", ext);
    let out_duct_path = target_dir.join(&out_duct_name);

    if raw_duct_path.exists() {
        let _ = fs::copy(&raw_duct_path, &out_duct_path);
        let root_duct_path = root.join(&out_duct_name);
        let _ = fs::copy(&raw_duct_path, &root_duct_path);
        if !prefix.is_empty() {
            let root_prefixed = root.join(format!("{}duct.{}", prefix, ext));
            let _ = fs::copy(&raw_duct_path, root_prefixed);
        }
        println!("[xtask] Deployed core router -> {}\n        {}", out_duct_path.display(), root_duct_path.display());
    }

    // Backend definitions: (flag, crate_raw_name, desired_module_name)
    let backends = [
        (config.build_gl, "glduct", "duct.gl"),
        (config.build_dx11, "dx11duct", "duct.dx11"),
        (config.build_dx12, "dx12duct", "duct.dx12"),
        (config.build_vk, "vkduct", "duct.vk"),
    ];

    for (enabled, crate_name, module_name) in backends {
        if !enabled {
            continue;
        }

        let raw_dll = target_dir.join(format!("{}{}.{}", prefix, crate_name, ext));
        let desired_dll_name = format!("{}.{}", module_name, ext);

        if !raw_dll.exists() {
            eprintln!(
                "[xtask][WARN] Expected compiled binary not found at {}",
                raw_dll.display()
            );
            continue;
        }

        // Copy to target/<profile>/renderer/duct.<api>.dll
        let dest_target = target_renderer.join(&desired_dll_name);
        fs::copy(&raw_dll, &dest_target).expect("Failed to copy DLL to target renderer directory");

        // Copy to root ./renderer/duct.<api>.dll
        let dest_local = local_renderer.join(&desired_dll_name);
        fs::copy(&raw_dll, &dest_local).expect("Failed to copy DLL to local renderer directory");

        // On Unix, also provide lib-prefixed aliases (e.g. libduct.gl.so / libduct.gl.dylib)
        if !prefix.is_empty() {
            let prefixed_name = format!("{}{}.{}", prefix, module_name, ext);
            let _ = fs::copy(&raw_dll, target_renderer.join(&prefixed_name));
            let _ = fs::copy(&raw_dll, local_renderer.join(&prefixed_name));
        }

        println!(
            "[xtask] Deployed backend [{}] -> \n        {}\n        {}",
            crate_name,
            dest_target.display(),
            dest_local.display()
        );
    }

    println!("[xtask] Deployment complete! Ready to run.");
}

fn get_platform_dll_spec() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("", "dll")
    } else if cfg!(target_os = "macos") {
        ("lib", "dylib")
    } else {
        ("lib", "so")
    }
}

fn workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap());

    // If running within xtask folder, workspace root is parent
    if manifest_dir.ends_with("xtask") {
        manifest_dir.parent().unwrap().to_path_buf()
    } else {
        manifest_dir
    }
}
