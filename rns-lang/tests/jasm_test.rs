use assert_cmd::cargo_bin_cmd;
use insta::with_settings;
use rstest::rstest;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SNAPSHOT_PATH: &str = "snapshots";

fn get_file_contents(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("Unable to read file")
}

fn get_hash(path: &Path) -> String {
    let bytes = fs::read(path).expect("Unable to read class file");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

fn get_relative_path_for_test(absolute_path: &Path) -> PathBuf {
    let cwd = std::env::current_dir().expect("Failed to get current working directory");
    absolute_path
        .strip_prefix(&cwd)
        .unwrap_or(absolute_path)
        .to_path_buf()
}

fn to_snapshot_name(path: &Path, flag: Option<&str>) -> String {
    let marker = Path::new("test_data/integration");
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

    let flag_suffix = flag
        .map(|f| format!("_{}", f.trim_start_matches('-').to_lowercase()))
        .unwrap_or_default();

    let base_name = format!("{}--{}", parent, stem);
    let full_name = if flag_suffix.is_empty() {
        base_name
    } else {
        format!("{}{}", base_name, flag_suffix)
    };
    format!("{}.snap", full_name.replace("/", "-").replace("--", "-"))
}

struct TempClassFile(PathBuf);

impl TempClassFile {
    fn new(source_path: &Path, flag: Option<&str>) -> Self {
        let temp_dir = std::env::temp_dir();
        let stem = source_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let flag_suffix = flag
            .map(|f| format!("_{}", f.trim_start_matches('-')))
            .unwrap_or_default();
        let class_file = temp_dir.join(format!(
            "test_output_{}{}_{}_{}.class",
            stem,
            flag_suffix,
            std::process::id(),
            rand::random::<u16>()
        ));
        fs::remove_file(&class_file).ok();
        Self(class_file)
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

#[rstest]
#[case("", None)]
#[case("--wasm", Some("--wasm"))]
#[case("--werror", Some("--werror"))]
fn test_integration(
    #[case] _flag: &str,
    #[case] flag: Option<&str>,
    #[base_dir = "test_data/integration/"]
    #[files("**/*.ja")]
    path: PathBuf,
) {
    let class_file = TempClassFile::new(&path, flag);

    let mut assemble_cmd = cargo_bin_cmd!("jasm");
    assemble_cmd
        .arg("asm")
        .arg(&path)
        .arg("--output")
        .arg(class_file.path());
    if let Some(f) = flag {
        assemble_cmd.arg(f);
    }
    let assemble_output = assemble_cmd.output().expect("Failed to assemble");

    let stdout = String::from_utf8_lossy(&assemble_output.stdout);
    let stderr = String::from_utf8_lossy(&assemble_output.stderr);

    assert!(
        stdout.trim().is_empty(),
        "Expected stdout to be empty, but got:\n{}",
        stdout
    );

    let (disassembled, hash) = if assemble_output.status.success() {
        let h = get_hash(class_file.path());

        let dis_output = {
            let mut cmd = cargo_bin_cmd!("jasm");
            cmd.arg("dis").arg(class_file.path());
            let output = cmd.output().expect("Failed to execute disassemble");
            String::from_utf8(output.stdout).expect("Failed to read disassemble output")
        };
        (dis_output, h)
    } else {
        ("not generated".to_string(), "not generated".to_string())
    };

    let original_contents = get_file_contents(&path);

    let combined = format!(
        "----- DISASSEMBLED -----\n{}\n----- INPUT -----\n{}\n----- STDERR -----\n{}\n----- HASH -----\n{}",
        disassembled.trim_end(),
        original_contents.trim_end(),
        stderr.trim_end(),
        hash
    );

    with_settings!(
        {
            snapshot_path => SNAPSHOT_PATH,
            prepend_module_to_snapshot => false,
        },
        {
            insta::assert_snapshot!(to_snapshot_name(&path, flag), &combined);
        }
    );
}
