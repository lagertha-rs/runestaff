use assert_cmd::cargo::cargo_bin_cmd;
use insta::with_settings;
use rstest::rstest;
use std::path::{Path, PathBuf};

const DISPLAY_SNAPSHOT_PATH: &str = "../snapshots";

fn get_relative_path_for_test(absolute_path: &Path) -> PathBuf {
    let cwd = std::env::current_dir().expect("Failed to get current working directory");

    absolute_path
        .strip_prefix(&cwd)
        .unwrap_or(absolute_path)
        .to_path_buf()
}

fn to_snapshot_name(path: &Path) -> String {
    let marker = Path::new("tests/testdata");
    let components = path.components().collect::<Vec<_>>();

    // Find index of "tests/testdata"
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

    // Remove ".ja" extension if present
    new_path.set_extension("");

    new_path
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("-")
}

fn get_file_contents(path: &PathBuf) -> String {
    std::fs::read_to_string(path).expect("Unable to read file")
}

#[rstest]
#[trace]
fn error_cases(
    #[base_dir = "tests/testdata/"]
    #[files("**/*.ja")]
    path: PathBuf,
) {
    // given
    let relative_path = get_relative_path_for_test(&path);
    let mut cmd = cargo_bin_cmd!("jasm");
    cmd.arg(&relative_path);
    let file_contents = get_file_contents(&path);

    // when
    let output = cmd.assert().failure().get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined = format!(
        "----- INPUT  -----\n{}\n----- STDOUT -----\n{}\n----- STDERR -----\n{}",
        file_contents,
        stdout.trim_end(),
        stderr.trim_end()
    );

    // then
    with_settings!(
        {
            snapshot_path => DISPLAY_SNAPSHOT_PATH,
            prepend_module_to_snapshot => false,
        },
        {
            insta::assert_snapshot!(to_snapshot_name(&path), &combined);
        }
    );
}
