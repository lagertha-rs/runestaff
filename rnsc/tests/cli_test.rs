//! Integration tests for `rnsc` CLI flags.
//!
//! Each `.rns` fixture under `test_data/cli_integration/` is assembled twice:
//!   1. with the default flag set (no `-q`)
//!   2. with `-q` / `--quiet`
//!
//! The snapshot captures INPUT, STDOUT, STDERR, exit status and the produced
//! class file hash for both invocations, so the contrast between the two runs
//! is visible in a single snapshot.

use assert_cmd::Command;
use insta::with_settings;
use rstest::rstest;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SNAPSHOT_PATH: &str = "snapshots";
const MARKER: &str = "test_data/cli_integration";

fn normalize_stderr_paths(stderr: &str, input_path: &Path) -> String {
    if let Ok(absolute) = input_path.canonicalize() {
        let abs_str = absolute.to_string_lossy();
        let mut normalized = stderr.to_string();

        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let rel_path = if let Ok(rel) = absolute.strip_prefix(&cwd) {
            rel.to_string_lossy()
        } else {
            input_path.to_string_lossy()
        };

        normalized = normalized.replace(&*abs_str, &rel_path);

        let abs_with_colon = format!("{}:", abs_str);
        let rel_with_colon = format!("{}:", rel_path);
        normalized = normalized.replace(&abs_with_colon, &rel_with_colon);

        return normalized;
    }

    stderr.to_string()
}

fn to_snapshot_name(path: &Path) -> String {
    let marker = Path::new(MARKER);
    let components = path.components().collect::<Vec<_>>();
    let marker_parts = marker.components().collect::<Vec<_>>();
    let idx = components
        .windows(marker_parts.len())
        .position(|window| window == marker_parts)
        .expect("Marker path not found in the given path");

    let after = &components[idx + marker_parts.len()..];
    let mut new_path = PathBuf::new();
    for c in after {
        new_path.push(c);
    }
    new_path.set_extension("");

    let stem = new_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let parent = new_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let base_name = format!("cli-{}--{}", parent, stem);
    base_name.replace("/", "-").replace("--", "-")
}

fn hash_of(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            format!("{:x}", hasher.finalize())
        }
        Err(_) => "not generated".to_string(),
    }
}

fn find_class_file_recursive(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_class_file_recursive(&path) {
                    return Some(found);
                }
            } else if path.extension().is_some_and(|ext| ext == "class") {
                return Some(path);
            }
        }
    }
    None
}

struct RunResult {
    stdout: String,
    stderr: String,
    success: bool,
    hash: String,
}

fn run_assembly(input: &Path, output_dir: &Path, extra_args: &[&str]) -> RunResult {
    let mut cmd = Command::cargo_bin("rnsc").expect("rnsc binary not found");
    cmd.arg("asm").arg(input).arg("-d").arg(output_dir);
    for arg in extra_args {
        cmd.arg(arg);
    }
    let output_res = cmd.output().expect("Failed to execute rnsc");
    let stdout = String::from_utf8_lossy(&output_res.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output_res.stderr).into_owned();
    let success = output_res.status.success();

    // Find the generated class file (recursively for package directories)
    let hash = if success {
        if let Some(class_file) = find_class_file_recursive(output_dir) {
            let hash = hash_of(&class_file);
            fs::remove_file(&class_file).ok();
            hash
        } else {
            "not generated".to_string()
        }
    } else {
        "not generated".to_string()
    };

    RunResult {
        stdout,
        stderr,
        success,
        hash,
    }
}

fn trim(s: &str) -> &str {
    s.trim_end_matches('\n')
}

fn build_snapshot(input_path: &Path, default: &RunResult, quiet: &RunResult) -> String {
    let original = fs::read_to_string(input_path).expect("Unable to read fixture");
    let normalized_default = normalize_stderr_paths(&default.stderr, input_path);
    let normalized_quiet = normalize_stderr_paths(&quiet.stderr, input_path);
    format!(
        "----- INPUT -----\n{}\n\
         ----- DEFAULT -----\n\
         stdout: {stdout_default}\n\
         stderr: {stderr_default}\n\
         exit: {exit_default}\n\
         hash: {hash_default}\n\
         ----- QUIET (-q) -----\n\
         stdout: {stdout_quiet}\n\
         stderr: {stderr_quiet}\n\
         exit: {exit_quiet}\n\
         hash: {hash_quiet}\n",
        trim(&original),
        stdout_default = trim(&default.stdout),
        stderr_default = trim(&normalized_default),
        exit_default = default.success,
        hash_default = default.hash,
        stdout_quiet = trim(&quiet.stdout),
        stderr_quiet = trim(&normalized_quiet),
        exit_quiet = quiet.success,
        hash_quiet = quiet.hash,
    )
}

#[rstest]
fn test_cli_integration(
    #[base_dir = "test_data/cli_integration/"]
    #[files("**/*.rns")]
    path: PathBuf,
) {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let default_dir = env::temp_dir().join(format!(
        "rnsc_cli_test_{}_default_{}",
        stem,
        rand::random::<u16>()
    ));
    let quiet_dir = env::temp_dir().join(format!(
        "rnsc_cli_test_{}_quiet_{}",
        stem,
        rand::random::<u16>()
    ));

    fs::create_dir_all(&default_dir).ok();
    fs::create_dir_all(&quiet_dir).ok();

    let default = run_assembly(&path, &default_dir, &[]);
    let quiet = run_assembly(&path, &quiet_dir, &["-q"]);

    let snapshot = build_snapshot(&path, &default, &quiet);

    fs::remove_dir_all(&default_dir).ok();
    fs::remove_dir_all(&quiet_dir).ok();

    with_settings!(
        {
            snapshot_path => SNAPSHOT_PATH,
            prepend_module_to_snapshot => false,
        },
        {
            insta::assert_snapshot!(to_snapshot_name(&path), &snapshot);
        }
    );
}
