# Dependency List Tool (`list_deps.py`)

This script scans a monorepo's packages directory and generates a CSV report of all third-party dependencies (Rust, NPM, Maven) for each package, including license and description metadata.

## Usage

1. **Create and activate a virtual environment:**

```bash
python3 -m venv .venv
source .venv/bin/activate
```

2. **Install dependencies:**

```bash
pip install -r requirements.txt
```

> **Note:** Required packages: `requests`, `tomli`

3. **Run the script:**

```bash
python list_deps.py ../packages -o ../output/dependencies.csv
```

- The first argument is the path to the packages directory (e.g., `../packages`).
- The `-o`/`--output` argument specifies the output CSV file (e.g., `../output/dependencies.csv`).

## Output

- The script will create a CSV file with columns: `Package`, `Dependency`, `Version`, `License`, `Description`.
- By default, output is written to `dependencies.csv` in the current directory. For this project, use the `output/` directory.

## Example

```bash
python list_deps.py ../packages -o ../output/dependencies.csv
```

## Wrapper Script

A wrapper script (`generate-dependency-report.sh`) is provided to automate running this tool with the correct arguments and environment.

---

**See also:** The VS Code task `generate.dependency.report` for one-click generation from the command palette.
