# fastsave Manual

fastsave is a tool for executing scripts (Python, Shell) while automatically saving their outputs and execution details in a structured way.

## Basic Usage

```bash
# Basic usage
fastsave run_simulation.py

# With script arguments
fastsave run_simulation.py --rows 3 --cols 4

# With both fastsave and script arguments
fastsave -a custom_archive run_simulation.py --rows 3 --cols 4

# Using -- to explicitly separate fastsave args from script args
fastsave -m "test message" run_simulation.py -- --rows 3 --cols 4
```

## Arguments

### fastsave Arguments

- `<script_path>`: (Required) Path to the script to execute
- `-a, --archive-dir <DIR>`: Directory to store results (default: "archive")
- `-m, --message <MESSAGE>`: Optional message to include with the results
- `--no-subfolder`: Store results directly in archive directory without creating a timestamped subfolder

## Output Structure

By default, fastsave creates a structured output directory:

archive/
└── YYYY-MM-DD_script-name_runN/
├── fastsave.json # Execution details and results
└── [script outputs] # Any files created by the script

The directory name format is:
- `YYYY-MM-DD`: Current date
- `script-name`: Name of the executed script (without extension)
- `runN`: Run number, automatically incremented for each run

### fastsave.json

The JSON file contains:
- Script information (path, type)
- Execution timestamps (start, end)
- Duration in milliseconds
- Exit code
- Standard output and error
- Optional message

```json
json
{
"script_path": "run_simulation.py",
"script_type": "python",
"start_time": "2024-01-17T15:30:00Z",
"end_time": "2024-01-17T15:30:01Z",
"duration_ms": 1000,
"exit_code": 0,
"stdout": "Simulation completed successfully",
"stderr": "",
"message": "Test run with parameters X and Y"
}
````

## Supported Script Types

Currently supported script types (detected by file extension):
- Python (`.py`)
- Shell (`.sh`)

## Script Requirements

Scripts should accept an `--output_dir` argument where they will write their output files. Example Python script:

```python
import argparse
from pathlib import Path
def main():
parser = argparse.ArgumentParser()
parser.add_argument('--output_dir', default='')
args = parser.parse_args()
output_path = Path(args.output_dir)
# Write outputs to output_path
with (output_path/'results.txt').open('w') as f:
```

## Installation

### Prerequisites
- Rust toolchain (1.70.0 or later)
- Cargo package manager

### Building from Source
1. Clone the repository
2. Run the following commands:
```bash
cargo build --release
cargo install --path .
```

## Error Handling

fastsave will:
- Create necessary directories if they don't exist
- Detect script type from file extension
- Capture and report script execution errors
- Save execution details even if the script fails
## Error Handling

fastsave will:
- Create necessary directories if they don't exist
- Detect script type from file extension
- Capture and report script execution errors
- Save execution details even if the script fails

## License

This project is licensed under the MIT License - see the LICENSE file for details.
