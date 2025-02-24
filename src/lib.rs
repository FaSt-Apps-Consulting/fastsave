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
use serde_yaml;
use std::process::Stdio;
use std::io::{self, Write, BufRead, BufReader};
use shellexpand;

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

    /// Override the interpreter for the script
    #[arg(short = 'i', long = "interpreter")]
    pub interpreter: Option<String>,

    /// Override the config file path
    #[arg(short = 'c', long = "config")]
    pub config_path: Option<String>,
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
    pub command_string: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FastsaveConfig {
    interpreters: HashMap<String, String>,
}

impl FastsaveConfig {
    pub fn load_with_config_path(config_path: Option<&str>) -> Self {
        // If config path is provided, try it first
        if let Some(path) = config_path {
            let expanded_path = shellexpand::tilde(path).to_string();
            println!("Debug: Trying to load config from custom path: {}", expanded_path);
            if let Ok(contents) = fs::read_to_string(&expanded_path) {
                println!("Debug: Found config file with contents:\n{}", contents);
                match serde_yaml::from_str(&contents) {
                    Ok(config) => {
                        println!("Debug: Successfully parsed config");
                        return config;
                    }
                    Err(e) => println!("Debug: Failed to parse custom config: {}", e),
                }
            }
        }

        // Fall back to default locations if custom path fails or isn't provided
        let config_paths = [
            "fastsave.yaml",  // Current directory
            "~/.config/fastsave/config.yaml", // User config directory
        ];

        for path in config_paths.iter() {
            let expanded_path = shellexpand::tilde(path).to_string();
            println!("Debug: Trying to load config from: {}", expanded_path);
            if let Ok(contents) = fs::read_to_string(&expanded_path) {
                println!("Debug: Found config file with contents:\n{}", contents);
                match serde_yaml::from_str(&contents) {
                    Ok(config) => {
                        println!("Debug: Successfully parsed config");
                        return config;
                    }
                    Err(e) => println!("Debug: Failed to parse config: {}", e),
                }
            }
        }
        
        println!("Debug: No config file found, using default config");
        FastsaveConfig::default()
    }

    // Add convenience method that maintains backward compatibility
    pub fn load() -> Self {
        Self::load_with_config_path(None)
    }

    pub fn get_interpreter(&self, extension: &str) -> Option<&String> {
        // Remove the leading dot if present and convert to lowercase
        let ext = extension.trim_start_matches('.').to_lowercase();
        let result = self.interpreters.get(&ext);
        println!("Debug: Looking up interpreter for extension '{}', found: {:?}", ext, result);
        result
    }
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
    let mut current = if start_path.is_absolute() {
        start_path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(start_path)
    };
    
    let mut highest_git_root = None;

    while let Some(parent) = current.parent() {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            highest_git_root = Some(current.clone());
        }
        current = parent.to_path_buf();
    }

    highest_git_root
}

fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_git_info(script_path: &str) -> Option<GitInfo> {
    let script_path = Path::new(script_path);
    let script_dir = if script_path.is_absolute() {
        script_path.parent()?.to_path_buf()
    } else {
        let current_dir = std::env::current_dir().ok()?;
        current_dir.join(script_path).parent()?.to_path_buf()
    };
    
    let repo_root = find_git_root(&script_dir)?;
    
    // Print debug information
    println!("Debug: Found git root at: {}", repo_root.display());
    
    let result = (|| -> Result<GitInfo, Box<dyn Error>> {
        let branch = run_git_command(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
        let commit_hash = run_git_command(&repo_root, &["rev-parse", "HEAD"])?;
        
        // Handle remote URL more gracefully
        let remote_url = match run_git_command(&repo_root, &["config", "--get", "remote.origin.url"]) {
            Ok(url) if !url.is_empty() => url,
            _ => String::from("No remote URL found"),
        };
        
        let status_output = run_git_command(&repo_root, &["status", "--porcelain"])?;
        let is_dirty = !status_output.is_empty();
        let uncommitted_changes = status_output
            .lines()
            .filter(|line| !line.is_empty())
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

    match result {
        Ok(info) => Some(info),
        Err(e) => {
            eprintln!("Debug: Error getting git info: {}", e);
            None
        }
    }
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

pub fn execute_script(script_path: &str, output_dir: &str, message: Option<String>, script_args: &[String], interpreter_override: Option<&String>, config_path: Option<&str>) -> Result<ExecutionResult, Box<dyn Error>> {
    let start_time = SystemTime::now();
    let start_datetime = DateTime::<Utc>::from(start_time);

    let git_info = get_git_info(script_path);

    let path = Path::new(script_path);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .ok_or("Unable to determine script type: no file extension")?;
    
    let program = if let Some(interpreter) = interpreter_override {
        interpreter.clone()
    } else {
        let config = FastsaveConfig::load_with_config_path(config_path);
        if let Some(interpreter) = config.get_interpreter(extension) {
            interpreter.to_string()
        } else {
            // Fall back to built-in defaults
            match extension.to_lowercase().as_str() {
                "py" => "python3".to_string(),
                "sh" => "sh".to_string(),
                "jl" => "julia".to_string(),
                "m" => "matlab".to_string(),
                _ => return Err(format!("Unsupported script type: {}", extension).into()),
            }
        }
    };

    // Build command string for logging and saving
    let command_string = format!("{} {}", 
        program,
        script_path
    );

    // Print the command before executing
    println!("Fastsave executes:\n{}", command_string);
    io::stdout().flush()?;

    // Build command with stdio configuration
    let mut cmd = Command::new(program);
    cmd.arg(script_path)
        .arg("--output_dir")
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // Add any additional script arguments
    for arg in script_args {
        cmd.arg(arg);
    }

    // Spawn the command
    let mut child = cmd.spawn()?;
    
    // Get handles to stdout and stderr
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Capture output while also displaying it
    let mut captured_stdout = String::new();
    let mut captured_stderr = String::new();

    // Create separate threads for stdout and stderr
    let stdout_handle = std::thread::spawn(move || {
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
                io::stdout().flush().unwrap();
                captured_stdout.push_str(&line);
                captured_stdout.push('\n');
            }
        }
        captured_stdout
    });

    let stderr_handle = std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                eprintln!("{}", line);
                io::stderr().flush().unwrap();
                captured_stderr.push_str(&line);
                captured_stderr.push('\n');
            }
        }
        captured_stderr
    });

    // Wait for the command to complete
    let status = child.wait()?;

    // Get the captured output
    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    let end_time = SystemTime::now();
    let end_datetime = DateTime::<Utc>::from(end_time);
    let duration = end_time.duration_since(start_time)?;

    let result = ExecutionResult {
        script_path: script_path.to_string(),
        start_time: start_datetime,
        end_time: end_datetime,
        duration_ms: duration.as_millis() as u64,
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        message,
        git_info,
        file_hashes: HashMap::new(),
        command_string,
    };

    Ok(result)
}

pub fn run_script(cli: &Cli) -> Result<String, Box<dyn Error>> {
    let output_dir = get_output_dir(cli)?;
    let output_file = Path::new(&output_dir).join("fastsave.yaml");

    let mut result = execute_script(
        &cli.script, 
        &output_dir, 
        cli.message.clone(), 
        &cli.script_args,
        cli.interpreter.as_ref(),
        cli.config_path.as_deref(),
    )?;

    // Calculate hashes for all generated files
    result.file_hashes = get_file_hashes(Path::new(&output_dir))?;

    // Save results to YAML file instead of JSON
    let yaml = serde_yaml::to_string(&result)?;
    fs::write(&output_file, yaml)?;

    Ok(output_dir)
} 