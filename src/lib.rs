use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::SystemTime;
use std::error::Error;
use clap::Parser;
use chrono::{DateTime, Utc, Local};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use std::io::Read;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the script to execute
    pub script: String,

    /// Archive directory path
    #[arg(short = 'a', long = "archive-dir", default_value = "archive")]
    pub archive_dir: String,

    /// Optional message to include in the results
    #[arg(short = 'm', long = "message")]
    pub message: Option<String>,

    /// Disable subfolder creation in archive directory
    #[arg(long = "no-subfolder")]
    pub no_subfolder: bool,

    /// Additional arguments to pass to the script
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub script_args: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_root: String,
    pub branch: String,
    pub commit_hash: String,
    pub remote_url: String,
    pub is_dirty: bool,
    pub uncommitted_changes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExecutionResult {
    pub script_path: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub message: Option<String>,
    pub git_info: Option<GitInfo>,
    pub file_hashes: HashMap<String, String>,
}

pub fn get_script_basename(script_path: &str) -> String {
    Path::new(script_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

pub fn get_next_run_number(base_dir: &str, script_name: &str, date: &str) -> u32 {
    if let Ok(entries) = fs::read_dir(base_dir) {
        let prefix = format!("{}_{}_run", date, script_name);
        
        entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(&prefix))
            .filter_map(|name| name.strip_prefix(&prefix).and_then(|n| n.parse::<u32>().ok()))
            .max()
            .map_or(1, |max| max + 1)
    } else {
        1
    }
}

pub fn create_run_dir(base_dir: &str, script_path: &str) -> Result<String, Box<dyn Error>> {
    fs::create_dir_all(base_dir)?;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let script_name = get_script_basename(script_path);
    let run_number = get_next_run_number(base_dir, &script_name, &date);
    
    let dir_name = format!("{}_{}_run{}", date, script_name, run_number);
    let dir_path = Path::new(base_dir).join(dir_name);
    
    fs::create_dir_all(&dir_path)?;
    
    Ok(dir_path.to_string_lossy().into_owned())
}

pub fn get_output_dir(cli: &Cli) -> Result<String, Box<dyn Error>> {
    if cli.no_subfolder {
        fs::create_dir_all(&cli.archive_dir)?;
        Ok(cli.archive_dir.clone())
    } else {
        create_run_dir(&cli.archive_dir, &cli.script)
    }
}

fn find_git_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = start_path.to_path_buf();
    while current.parent().is_some() {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Some(current);
        }
        current = current.parent().unwrap().to_path_buf();
    }
    None
}

fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ).into())
    }
}

fn get_git_info(script_path: &str) -> Option<GitInfo> {
    let script_dir = Path::new(script_path).parent()?;
    let repo_root = find_git_root(script_dir)?;
    
    let result = (|| -> Result<GitInfo, Box<dyn Error>> {
        let branch = run_git_command(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
        let commit_hash = run_git_command(&repo_root, &["rev-parse", "HEAD"])?;
        let remote_url = run_git_command(&repo_root, &["config", "--get", "remote.origin.url"])
            .unwrap_or_else(|_| String::from("No remote URL found"));
        
        let status_output = run_git_command(&repo_root, &["status", "--porcelain"])?;
        let is_dirty = !status_output.is_empty();
        let uncommitted_changes = status_output
            .lines()
            .map(|line| line.to_string())
            .collect();

        Ok(GitInfo {
            repo_root: repo_root.to_string_lossy().into_owned(),
            branch,
            commit_hash,
            remote_url,
            is_dirty,
            uncommitted_changes,
        })
    })();

    result.ok()
}

fn calculate_file_hash(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    Ok(format!("{:x}", hasher.finalize()))
}

fn get_file_hashes(dir: &Path) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut hashes = HashMap::new();
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let relative_path = path.strip_prefix(dir)?;
            let hash = calculate_file_hash(&path)?;
            hashes.insert(relative_path.to_string_lossy().to_string(), hash);
        }
    }
    
    Ok(hashes)
}

pub fn execute_script(script_path: &str, output_dir: &str, message: Option<String>, script_args: &[String]) -> Result<ExecutionResult, Box<dyn Error>> {
    let start_time = SystemTime::now();
    let start_datetime = DateTime::<Utc>::from(start_time);

    // let script_type = get_script_type(script_path)?;
    let git_info = get_git_info(script_path);

    let path = Path::new(script_path);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .ok_or("Unable to determine script type: no file extension")?;
    
    let program = match extension.to_lowercase().as_str() {
        "py" => "python",
        "sh" => "sh",
        "jl" => "julia",
        "m" => "matlab",
        _ => return Err("Unsupported script type".into()),
    };

    // Build command with all arguments
    let mut cmd = Command::new(program);
    cmd.arg(script_path)
        .arg("--output_dir")
        .arg(output_dir);
    
    // Add any additional script arguments
    for arg in script_args {
        cmd.arg(arg);
    }

    let output = cmd.output()?;

    let end_time = SystemTime::now();
    let end_datetime = DateTime::<Utc>::from(end_time);
    let duration = end_time.duration_since(start_time)?;

    let result = ExecutionResult {
        script_path: script_path.to_string(),
        start_time: start_datetime,
        end_time: end_datetime,
        duration_ms: duration.as_millis() as u64,
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        message,
        git_info,
        file_hashes: HashMap::new(),
    };

    Ok(result)
}

pub fn run_script(cli: &Cli) -> Result<String, Box<dyn Error>> {
    // Get output directory
    let output_dir = get_output_dir(cli)?;
    let output_file = Path::new(&output_dir).join("fastsave.json");

    // Execute script with additional arguments
    let mut result = execute_script(&cli.script, &output_dir, cli.message.clone(), &cli.script_args)?;

    // Calculate hashes for all generated files
    result.file_hashes = get_file_hashes(Path::new(&output_dir))?;

    // Save results to JSON file
    let json = serde_json::to_string_pretty(&result)?;
    fs::write(&output_file, json)?;

    Ok(output_dir)
} 