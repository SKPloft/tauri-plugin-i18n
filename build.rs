use std::{
    env, fs,
    path::{Path, PathBuf},
};

const COMMANDS: &[&str] = &[
    "load_translations",
    "translate",
    "set_locale",
    "get_locale",
    "get_available_locales",
];

fn main() {
    // Only bundle locales if we can find them (i.e., when used as dependency)
    // Skip during `cargo publish` or standalone builds
    if should_bundle_locales() {
        bundle_locales();
    } else {
        // Generate empty bundled_locales.rs for standalone builds
        generate_empty_bundled_locales();
    }
    tauri_plugin::Builder::new(COMMANDS).build();
}

fn should_bundle_locales() -> bool {
    let out_dir = env::var("OUT_DIR").unwrap();

    // If TAURI_I18N_LOCALES_PATH is set explicitly, we always try to bundle
    if env::var("TAURI_I18N_LOCALES_PATH").is_ok() {
        return true;
    }

    find_workspace_root(Path::new(&out_dir)).is_some()
}

fn generate_empty_bundled_locales() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bundled_locales.rs");

    println!("cargo:warning=No locales found - generating empty bundle (this is normal during cargo publish)");

    let code = "pub fn get_bundled_data() -> Vec<(&'static str, &'static str, &'static str)> {\n    vec![]\n}\n";

    fs::write(dest_path, code).expect("Failed to write bundled_locales.rs");
}

/// Find the workspace root and return the parent directory of the `src-tauri` directory.
/// This is used as the base for resolving relative locale paths.
fn find_workspace_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir;
    while let Some(parent) = current.parent() {
        // Check if this dir contains a Cargo.toml with [workspace]
        let cargo_toml = parent.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    // Now search recursively for src-tauri under this workspace root
                    if let Some(src_tauri_parent) = find_src_tauri(parent) {
                        println!(
                            "cargo:info=Found src-tauri parent: {}",
                            src_tauri_parent.display()
                        );
                        return Some(src_tauri_parent);
                    }
                }
            }
        }
        current = parent;
    }
    None
}

fn find_src_tauri(root: &Path) -> Option<PathBuf> {
    // BFS/DFS to find a `src-tauri` directory anywhere under root
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let src_tauri = dir.join("src-tauri");
        if src_tauri.exists() && src_tauri.is_dir() {
            return Some(dir);
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden dirs, target, node_modules
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "target" && name != "node_modules" {
                        stack.push(path);
                    }
                }
            }
        }
    }
    None
}

fn resolve_locales_path() -> PathBuf {
    println!("cargo:rerun-if-env-changed=TAURI_I18N_LOCALES_PATH");
    println!("cargo:rerun-if-env-changed=TAURI_I18N_PROJECT_DIR");

    // 1. Check for explicit locales path (absolute or relative to workspace)
    if let Ok(explicit_path) = env::var("TAURI_I18N_LOCALES_PATH") {
        let path = Path::new(&explicit_path);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            // Relative to the project directory (or workspace root as fallback)
            let base = resolve_project_dir();
            base.join(path)
        };

        println!(
            "cargo:info=Using explicit locales path: {}",
            resolved.display()
        );

        if !resolved.exists() {
            panic!(
                "TAURI_I18N_LOCALES_PATH is set but path does not exist: {}",
                resolved.display()
            );
        }

        return resolved;
    }

    // 2. Fallback: find src-tauri/locales under workspace
    let out_dir = env::var("OUT_DIR").unwrap();
    let workspace_root = find_workspace_root(Path::new(&out_dir))
        .expect("Could not find workspace root. Set TAURI_I18N_LOCALES_PATH or TAURI_I18N_PROJECT_DIR to point at your project.");

    let locales_path = workspace_root.join("src-tauri").join("locales");
    println!(
        "cargo:info=Using auto-detected locales path: {}",
        locales_path.display()
    );
    locales_path
}

/// Resolve the base directory for relative paths.
///
/// Uses `TAURI_I18N_PROJECT_DIR` if set; otherwise falls back to
/// finding the workspace root + src-tauri parent.
fn resolve_project_dir() -> PathBuf {
    if let Ok(project_dir) = env::var("TAURI_I18N_PROJECT_DIR") {
        let path = Path::new(&project_dir);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            // Relative to current working directory
            env::current_dir().unwrap().join(path)
        };

        println!(
            "cargo:info=Using project directory: {}",
            resolved.display()
        );

        if !resolved.exists() {
            panic!(
                "TAURI_I18N_PROJECT_DIR is set but path does not exist: {}",
                resolved.display()
            );
        }

        return resolved;
    }

    // Fallback: find workspace root + src-tauri parent
    let out_dir = env::var("OUT_DIR").unwrap();
    find_workspace_root(Path::new(&out_dir))
        .expect("Could not find project root. Set TAURI_I18N_PROJECT_DIR to point at your project directory.")
}

fn bundle_locales() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bundled_locales.rs");

    println!("cargo:info=OUT_DIR: {}", out_dir);

    let locales_path = resolve_locales_path();

    println!(
        "cargo:info=Looking for locales at: {}",
        locales_path.display()
    );

    if !locales_path.exists() {
        panic!(
            "Locales directory does not exist: {}",
            locales_path.display()
        );
    }

    println!("cargo:rerun-if-changed={}", locales_path.display());

    let mut code = String::from(
        "pub fn get_bundled_data() -> Vec<(&'static str, &'static str, &'static str)> {\n    vec![\n"
    );

    match fs::read_dir(&locales_path) {
        Ok(entries) => {
            let mut count = 0;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let (Some(stem), Some(ext)) = (
                        path.file_stem().and_then(|s| s.to_str()),
                        path.extension().and_then(|s| s.to_str()),
                    ) {
                        count += 1;
                        println!("cargo:info=  Bundling: {}.{}", stem, ext);
                        code.push_str(&format!(
                            "        ({:?}, {:?}, include_str!(r#\"{}\"#)),\n",
                            stem,
                            ext,
                            path.display()
                        ));
                    }
                }
            }
            println!("cargo:info=Successfully bundled {} locale file(s)", count);
        }
        Err(e) => {
            panic!(
                "Failed to read locales directory at {}: {}",
                locales_path.display(),
                e
            );
        }
    }

    code.push_str("    ]\n}\n");

    fs::write(dest_path, code).expect("Failed to write bundled_locales.rs");
}
