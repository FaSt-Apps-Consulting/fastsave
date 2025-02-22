# FaStSave

A tool for executing and monitoring scripts with JSON output.

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
```

## Configuration

You can configure interpreter mappings in either:
- `./fastsave.yaml` (current directory)
- `~/.config/fastsave/config.yaml` (user config)

Example configuration:
```yaml
interpreters:
  py: python3
  R: Rscript
  jl: julia
  m: octave
```

Default interpreter mappings:
- `.py` -> `python3`
- `.sh` -> `sh`
- `.jl` -> `julia`
- `.m` -> `matlab`

Fabian Stutzki

FaSt Apps & Consulting GmbH

2025