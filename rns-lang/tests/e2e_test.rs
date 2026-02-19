use assert_cmd::cargo_bin_cmd;
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

fn get_file_contents(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("Unable to read file")
}

struct TempClassFile(PathBuf);

impl TempClassFile {
    fn new() -> Self {
        let temp_dir = std::env::temp_dir();
        let class_file = temp_dir.join(format!("test_output_{}.class", std::process::id()));
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
fn e2e_roundtrip_assemble_disassemble(
    #[base_dir = "test_data/e2e/"]
    #[files("**/*.ja")]
    path: PathBuf,
) {
    let class_file = TempClassFile::new();

    {
        let mut cmd = cargo_bin_cmd!("jasm");
        cmd.arg("asm")
            .arg(&path)
            .arg("--output")
            .arg(class_file.path());
        cmd.assert().success();
    }

    let dis_output = {
        let mut cmd = cargo_bin_cmd!("jasm");
        cmd.arg("dis").arg(class_file.path());
        let output = cmd.output().expect("Failed to execute disassemble");
        String::from_utf8(output.stdout).expect("Failed to read disassemble output")
    };

    let original_contents = get_file_contents(&path);

    let normalize = |s: &str| {
        s.lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };

    assert_eq!(
        normalize(&dis_output),
        normalize(&original_contents),
        "Disassembled output should match original .ja file"
    );
}
