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

struct TempClassFile(PathBuf);

impl TempClassFile {
    fn new(stem: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rnsc_cli_test_{}_{}.class",
            stem,
            rand::random::<u16>()
        ));
        fs::remove_file(&path).ok();
        Self(path)
    }

    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempClassFile {
    fn drop(&mut self) {
        fs::remove_file(&self.0).ok();
    }
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

struct RunResult {
    stdout: String,
    stderr: String,
    success: bool,
    hash: String,
}

fn run_assembly(input: &Path, output: &Path, extra_args: &[&str]) -> RunResult {
    let mut cmd = Command::cargo_bin("rnsc").expect("rnsc binary not found");
    cmd.arg("asm").arg(input).arg("--output").arg(output);
    for arg in extra_args {
        cmd.arg(arg);
    }
    let output_res = cmd.output().expect("Failed to execute rnsc");
    let stdout = String::from_utf8_lossy(&output_res.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output_res.stderr).into_owned();
    let success = output_res.status.success();
    let hash = hash_of(output);
    fs::remove_file(output).ok();
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

    let default_out = TempClassFile::new(&format!("{stem}_default"));
    let quiet_out = TempClassFile::new(&format!("{stem}_quiet"));

    let default = run_assembly(&path, default_out.path(), &[]);
    let quiet = run_assembly(&path, quiet_out.path(), &["-q"]);

    let snapshot = build_snapshot(&path, &default, &quiet);

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