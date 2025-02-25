# FaStSave

A tool for executing and monitoring scripts with YAML output and metadata collection.

For full documentation, see [the manual](docs/manual.md).

## Quick Start

```bash
# Basic usage
fastsave run_simulation.py

# With script arguments
fastsave run_simulation.py --rows 3 --cols 4

# With both fastsave and script arguments
fastsave -a custom_archive run_simulation.py --rows 3 --cols 4

# Using -- to explicitly separate fastsave args from script args
fastsave -m "test message" run_simulation.py -- --rows 3 --cols 4

# Using a custom interpreter
fastsave -i python3.9 run_simulation.py

# Using a custom interpreter with configuration file
# First create fastsave.yaml:
#   interpreters:
#     py: python3.9
#     R: Rscript
# Then run:
fastsave run_analysis.R

# Using a custom config file path
fastsave -c /path/to/config.yaml run_simulation.py

# Using a custom config file with interpreter override
fastsave -c /path/to/config.yaml -i python3 run_simulation.py
```

## Arguments

- `<script>`: Path to the script to execute
- `-a, --archive-dir <DIR>`: Directory to store results (default: "archive")
- `-m, --message <MESSAGE>`: Optional message to include with the results
- `-i, --interpreter <INTERPRETER>`: Override the default interpreter
- `-c, --config <CONFIG>`: Use a custom configuration file
- `--no-subfolder`: Store results directly in archive directory
- `[script_args]...`: Additional arguments passed to the script

## Configuration

You can configure interpreter mappings in (in order of precedence):
1. Command line argument (`-i/--interpreter`)
2. Custom config file specified with `-c/--config`
3. Local config file (`./fastsave.yaml`)
4. User config file (`~/.config/fastsave/config.yaml`)
5. Built-in defaults

Example configuration:
```yaml
interpreters:
  py: python3
  R: Rscript
  jl: julia
  m: octave
```

Default interpreter mappings:
- `.py` -> `python`
- `.sh` -> `sh`
- `.jl` -> `julia`
- `.m` -> `matlab`

## Output

Results are saved in YAML format (`fastsave.yaml`) containing:
- Script execution details
- Start and end times
- Duration
- Exit code
- Standard output and error
- Git information (if script is in a git repository)
- File hashes of generated outputs
- Custom message (if provided)
- Command string used for execution

Fabian Stutzki

FaSt Apps & Consulting GmbH

2025