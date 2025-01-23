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
```

Fabian Stutzki

FaSt Apps & Consulting GmbH

2025