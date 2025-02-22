use std::fs;
use std::path::Path;
use tempfile::TempDir;
use fastsave::{Cli, ExecutionResult, run_script};
use std::process::Command;
use std::error::Error;
use std::path::PathBuf;

fn init_git_repo(dir: &Path) -> Result<(), Box<dyn Error>> {
    Command::new("git").args(&["init"]).current_dir(dir).output()?;
    Command::new("git").args(&["config", "user.name", "test"]).current_dir(dir).output()?;
    Command::new("git").args(&["config", "user.email", "test@example.com"]).current_dir(dir).output()?;
    Command::new("git").args(&["add", "."]).current_dir(dir).output()?;
    Command::new("git").args(&["commit", "-m", "Initial commit"]).current_dir(dir).output()?;
    Ok(())
}

fn create_nested_git_repos() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let root_dir = TempDir::new()?;
    
    // Create root git repo
    init_git_repo(root_dir.path())?;
    
    // Create nested structure
    let nested_path = root_dir.path().join("level1").join("level2");
    fs::create_dir_all(&nested_path)?;
    
    // Create script in nested directory
    let script_path = nested_path.join("test_script.py");
    fs::write(&script_path, "print('test')")?;
    
    // Add and commit the script
    Command::new("git")
        .current_dir(root_dir.path())
        .args(&["add", "."])
        .output()?;
    Command::new("git")
        .current_dir(root_dir.path())
        .args(&["commit", "-m", "Add test script"])
        .output()?;
    
    Ok((root_dir, script_path))
}

#[test]
fn test_git_info_collection() -> Result<(), Box<dyn Error>> {
    let (repo_dir, script_path) = create_nested_git_repos()?;
    
    // Test with absolute path
    let git_info = fastsave::get_git_info(script_path.to_str().unwrap())
        .expect("Should get git info");
    assert_eq!(
        fs::canonicalize(&git_info.repo_root)?,
        fs::canonicalize(repo_dir.path())?
    );
    assert!(!git_info.commit_hash.is_empty());
    assert!(!git_info.is_dirty);
    
    // Test with relative path
    let script_dir = script_path.parent().unwrap();
    std::env::set_current_dir(script_dir)?;
    let relative_git_info = fastsave::get_git_info("test_script.py")
        .expect("Should get git info");
    assert_eq!(
        fs::canonicalize(&relative_git_info.repo_root)?,
        fs::canonicalize(repo_dir.path())?
    );
    
    // Reset working directory
    std::env::set_current_dir(repo_dir.path())?;
    Ok(())
}

#[test]
fn test_basic_script_execution() {
    // Create a temporary directory for the archive
    let archive_dir = TempDir::new().unwrap();
    
    // Create a simple test script
    let script_content = r#"
import argparse
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output_dir', default='')
    args = parser.parse_args()
    
    output_path = Path(args.output_dir)
    with (output_path/'matrix.txt').open('w') as f:
        f.write('test matrix content')

if __name__ == '__main__':
    main()
"#;
    
    // Write the script to a temporary file
    let script_path = archive_dir.path().join("run_simulation.py");
    fs::write(&script_path, script_content).unwrap();
    // Create CLI args and run script
    let cli = Cli {
        script: script_path.to_string_lossy().to_string(),
        archive_dir: archive_dir.path().to_string_lossy().to_string(),
        message: None,
        no_subfolder: false,
        script_args: vec![],
    };

    let output_dir = run_script(&cli).unwrap();
    
    // Verify the output files exist
    let matrix_file = Path::new(&output_dir).join("matrix.txt");
    let fastsave_file = Path::new(&output_dir).join("fastsave.yaml");
    
    assert!(matrix_file.exists(), "matrix.txt should exist");
    assert!(fastsave_file.exists(), "fastsave.yaml should exist");
    
    // Verify the content of matrix.txt
    let matrix_content = fs::read_to_string(matrix_file).unwrap();
    assert_eq!(matrix_content, "test matrix content");
    
    // Verify the output directory name format
    assert!(output_dir.contains("run_simulation_run1"));
    
    // Verify the YAML content
    let yaml_content = fs::read_to_string(fastsave_file).unwrap();
    let saved_result: ExecutionResult = serde_yaml::from_str(&yaml_content).unwrap();
    assert_eq!(saved_result.exit_code, 0);
}

#[test]
fn test_script_with_arguments() {
    let archive_dir = TempDir::new().unwrap();
    
    // Create a test script that uses arguments
    let script_content = r#"
import argparse
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output_dir', default='')
    parser.add_argument('--rows', type=int, default=5)
    parser.add_argument('--cols', type=int, default=10)
    args = parser.parse_args()
    
    output_path = Path(args.output_dir)
    with (output_path/'matrix.txt').open('w') as f:
        f.write(f'Matrix size: {args.rows}x{args.cols}')

if __name__ == '__main__':
    main()
"#;
    
    let script_path = archive_dir.path().join("run_simulation.py");
    fs::write(&script_path, script_content).unwrap();
    
    let cli = Cli {
        script: script_path.to_string_lossy().to_string(),
        archive_dir: archive_dir.path().to_string_lossy().to_string(),
        message: None,
        no_subfolder: false,
        script_args: vec!["--rows".to_string(), "3".to_string(), "--cols".to_string(), "4".to_string()],
    };

    let output_dir = run_script(&cli).unwrap();
    
    // Verify the matrix content includes the passed arguments
    let matrix_file = Path::new(&output_dir).join("matrix.txt");
    let matrix_content = fs::read_to_string(matrix_file).unwrap();
    assert_eq!(matrix_content, "Matrix size: 3x4");
}

#[test]
fn test_custom_archive_directory() {
    // Create a temporary directory for the custom archive
    let custom_archive = TempDir::new().unwrap();
    
    // Create a simple test script
    let script_content = r#"
import argparse
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output_dir', default='')
    args = parser.parse_args()
    
    output_path = Path(args.output_dir)
    with (output_path/'test.txt').open('w') as f:
        f.write('test content')

if __name__ == '__main__':
    main()
"#;
    
    // Write the script to a temporary file
    let script_path = custom_archive.path().join("test_script.py");
    fs::write(&script_path, script_content).unwrap();
    
    let cli = Cli {
        script: script_path.to_string_lossy().to_string(),
        archive_dir: custom_archive.path().to_string_lossy().to_string(),
        message: None,
        no_subfolder: false,
        script_args: vec![],
    };

    let output_dir = run_script(&cli).unwrap();
    
    // Verify that the output directory is under our custom archive directory
    assert!(Path::new(&output_dir).starts_with(custom_archive.path()));
    
    // Verify the output file exists in the correct location
    let test_file = Path::new(&output_dir).join("test.txt");
    assert!(test_file.exists(), "test.txt should exist in custom archive directory");
}

#[test]
fn test_git_repository_info() {
    // Create a temporary directory for the test repository
    let repo_dir = TempDir::new().unwrap();
    
    // Initialize a git repository
    Command::new("git")
        .current_dir(repo_dir.path())
        .args(&["init"])
        .output()
        .unwrap();

    // Configure git user for commits
    Command::new("git")
        .current_dir(repo_dir.path())
        .args(&["config", "user.name", "Test User"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(repo_dir.path())
        .args(&["config", "user.email", "test@example.com"])
        .output()
        .unwrap();

    // Create a test script in the repository
    let script_content = r#"
import argparse
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output_dir', default='')
    args = parser.parse_args()
    
    output_path = Path(args.output_dir)
    with (output_path/'test.txt').open('w') as f:
        f.write('test content')

if __name__ == '__main__':
    main()
"#;
    
    let script_path = repo_dir.path().join("test_script.py");
    fs::write(&script_path, script_content).unwrap();

    // Add and commit the script
    Command::new("git")
        .current_dir(repo_dir.path())
        .args(&["add", "test_script.py"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(repo_dir.path())
        .args(&["commit", "-m", "Initial commit"])
        .output()
        .unwrap();

    // Create CLI args and run script
    let cli = Cli {
        script: script_path.to_string_lossy().to_string(),
        archive_dir: repo_dir.path().join("archive").to_string_lossy().to_string(),
        message: None,
        no_subfolder: false,
        script_args: vec![],
    };

    let output_dir = run_script(&cli).unwrap();
    
    // Read and parse the fastsave.yaml file
    let yaml_content = fs::read_to_string(Path::new(&output_dir).join("fastsave.yaml")).unwrap();
    let result: ExecutionResult = serde_yaml::from_str(&yaml_content).unwrap();

    // Verify Git information
    let git_info = result.git_info.expect("Git info should be present");
    
    assert!(git_info.repo_root.contains(repo_dir.path().to_string_lossy().as_ref()));
    assert!(!git_info.commit_hash.is_empty());
    assert!(!git_info.is_dirty);
    assert!(!git_info.branch.is_empty(), "Branch name should not be empty");
    assert!(git_info.uncommitted_changes.is_empty());

    // Test with uncommitted changes
    fs::write(repo_dir.path().join("new_file.txt"), "new content").unwrap();
    
    let output_dir = run_script(&cli).unwrap();
    let yaml_content = fs::read_to_string(Path::new(&output_dir).join("fastsave.yaml")).unwrap();
    let result: ExecutionResult = serde_yaml::from_str(&yaml_content).unwrap();
    
    let git_info = result.git_info.expect("Git info should be present");
    assert!(git_info.is_dirty);
    assert!(!git_info.uncommitted_changes.is_empty());
}

#[test]
fn test_file_hashes() {
    let archive_dir = TempDir::new().unwrap();
    
    // Create a test script that generates multiple files
    let script_content = r#"
import argparse
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output_dir', default='')
    args = parser.parse_args()
    
    output_path = Path(args.output_dir)
    # Create multiple files with different content
    with (output_path/'file1.txt').open('w') as f:
        f.write('content1')
    with (output_path/'file2.txt').open('w') as f:
        f.write('content2')

if __name__ == '__main__':
    main()
"#;
    
    let script_path = archive_dir.path().join("test_script.py");
    fs::write(&script_path, script_content).unwrap();
    
    let cli = Cli {
        script: script_path.to_string_lossy().to_string(),
        archive_dir: archive_dir.path().to_string_lossy().to_string(),
        message: None,
        no_subfolder: false,
        script_args: vec![],
    };

    let output_dir = run_script(&cli).unwrap();
    
    // Read and parse the fastsave.yaml file
    let yaml_content = fs::read_to_string(Path::new(&output_dir).join("fastsave.yaml")).unwrap();
    let result: ExecutionResult = serde_yaml::from_str(&yaml_content).unwrap();
    
    // Verify file hashes
    assert!(result.file_hashes.contains_key("file1.txt"));
    assert!(result.file_hashes.contains_key("file2.txt"));
    
    // Verify different content produces different hashes
    assert_ne!(
        result.file_hashes.get("file1.txt"),
        result.file_hashes.get("file2.txt")
    );
}

#[test]
fn test_custom_interpreter() {
    let archive_dir = TempDir::new().unwrap();
    
    // Create a test script that prints its interpreter
    let script_content = r#"
import argparse
import sys
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output_dir', default='')
    parser.add_argument('--interpreter', default='python')
    args = parser.parse_args()
    
    output_path = Path(args.output_dir)
    with (output_path/'interpreter_info.txt').open('w') as f:
        f.write(f'Interpreter: {sys.executable}')

if __name__ == '__main__':
    main()
"#;
    
    let script_path = archive_dir.path().join("test_script.py");
    fs::write(&script_path, script_content).unwrap();
    
    // Test with python3 interpreter
    let cli = Cli {
        script: script_path.to_string_lossy().to_string(),
        archive_dir: archive_dir.path().to_string_lossy().to_string(),
        message: None,
        no_subfolder: false,
        script_args: vec!["--interpreter".to_string(), "python3".to_string()],
    };

    let output_dir = run_script(&cli).unwrap();
    
    // Verify the output file exists
    let info_file = Path::new(&output_dir).join("interpreter_info.txt");
    assert!(info_file.exists(), "interpreter_info.txt should exist");
    
    // Read and verify the content
    let info_content = fs::read_to_string(info_file).unwrap();
    assert!(info_content.contains("Interpreter:"), "Should contain interpreter information");
    
    // Verify the execution was successful
    let yaml_content = fs::read_to_string(Path::new(&output_dir).join("fastsave.yaml")).unwrap();
    let result: ExecutionResult = serde_yaml::from_str(&yaml_content).unwrap();
    assert_eq!(result.exit_code, 0, "Script should execute successfully with custom interpreter");
}
