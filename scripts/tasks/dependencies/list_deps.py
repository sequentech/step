# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import os
import csv
import json
import subprocess
import xml.etree.ElementTree as ET
import argparse
from pathlib import Path
from urllib.parse import quote

# Third-party libraries required. Install with:
# pip install requests tomli
import requests
import tomli

# --- Configuration ---
MAVEN_NAMESPACES = {'m': 'http://maven.apache.org/POM/4.0.0'}

def fetch_rust_deps(package_path, package_name, writer):
    """Parses Cargo.toml and queries the crates.io API."""
    try:
        with open(package_path / "Cargo.toml", "rb") as f:
            cargo_data = tomli.load(f)
        
        dependencies = cargo_data.get("dependencies", {})
        print(f"  -> Found {len(dependencies)} Rust dependencies for '{package_name}'")

        for name, version_info in dependencies.items():
            version = version_info if isinstance(version_info, str) else version_info.get("version", "N/A")
            try:
                # Query crates.io API for metadata
                response = requests.get(f"https://crates.io/api/v1/crates/{quote(name)}", timeout=5)
                response.raise_for_status()
                api_data = response.json().get("crate", {})
                
                license = api_data.get("license", "N/A")
                description = (api_data.get("description", "") or "").replace("\n", " ").strip()
                writer.writerow([package_name, name, version, license, description])
            except requests.RequestException as e:
                print(f"    [WARN] Could not fetch metadata for Rust crate '{name}': {e}")
                writer.writerow([package_name, name, version, "Error", str(e)])

    except Exception as e:
        print(f"  [ERROR] Failed to process Cargo.toml for '{package_name}': {e}")

def fetch_npm_deps(package_path, package_name, writer):
    """Parses package.json and uses `npm view` for metadata."""
    try:
        with open(package_path / "package.json", "r", encoding="utf-8") as f:
            package_data = json.load(f)
            
        dependencies = package_data.get("dependencies", {})
        print(f"  -> Found {len(dependencies)} TypeScript dependencies for '{package_name}'")

        for name, version in dependencies.items():
            try:
                # `npm view --json` is fast and gets all info in one call
                result = subprocess.run(
                    ["npm", "view", name, "--json"],
                    capture_output=True, text=True, check=True, encoding="utf-8"
                )
                dep_data = json.loads(result.stdout)
                license = dep_data.get("license", "N/A")
                description = (dep_data.get("description", "") or "").replace("\n", " ").strip()
                writer.writerow([package_name, name, version, license, description])
            except (subprocess.CalledProcessError, json.JSONDecodeError) as e:
                print(f"    [WARN] Could not fetch metadata for npm package '{name}': {e}")
                writer.writerow([package_name, name, version, "Error", str(e)])

    except Exception as e:
        print(f"  [ERROR] Failed to process package.json for '{package_name}': {e}")

def fetch_maven_deps(package_path, package_name, writer):
    """Parses pom.xml and fetches dependency POMs from Maven Central."""
    try:
        tree = ET.parse(package_path / "pom.xml")
        root = tree.getroot()
        
        # Helper to find text in namespaced XML
        def find_text(element, path):
            node = element.find(path, MAVEN_NAMESPACES)
            return node.text.strip() if node is not None else None

        dependencies = root.findall(".//m:dependencies/m:dependency", MAVEN_NAMESPACES)
        print(f"  -> Found {len(dependencies)} Java dependencies for '{package_name}'")

        for dep in dependencies:
            groupId = find_text(dep, 'm:groupId')
            artifactId = find_text(dep, 'm:artifactId')
            version = find_text(dep, 'm:version')

            if not all([groupId, artifactId, version]) or "${" in version:
                continue
            
            dep_name = f"{groupId}:{artifactId}"
            group_path = groupId.replace('.', '/')
            pom_url = f"https://repo1.maven.org/maven2/{group_path}/{artifactId}/{version}/{artifactId}-{version}.pom"

            try:
                response = requests.get(pom_url, timeout=5)
                if response.status_code == 200:
                    dep_root = ET.fromstring(response.content)
                    license_node = find_text(dep_root, ".//m:licenses/m:license/m:name") or "N/A"
                    desc_node = find_text(dep_root, ".//m:description") or ""
                    description = desc_node.replace("\n", " ").strip()
                    writer.writerow([package_name, dep_name, version, license_node, description])
                else:
                    writer.writerow([package_name, dep_name, version, "Not Found", f"POM not found at {pom_url}"])
            except requests.RequestException as e:
                print(f"    [WARN] Could not fetch POM for '{dep_name}': {e}")
                writer.writerow([package_name, dep_name, version, "Error", str(e)])
    
    except ET.ParseError as e:
        print(f"  [ERROR] Failed to parse pom.xml for '{package_name}': {e}")


def main():
    """Main function to parse arguments, iterate through packages, and extract dependencies."""
    parser = argparse.ArgumentParser(
        description="Scan a directory of monorepo packages to extract third-party dependencies.",
        formatter_class=argparse.RawTextHelpFormatter
    )
    parser.add_argument(
        "packages_dir",
        help="Path to the directory containing the packages (e.g., 'packages')."
    )
    parser.add_argument(
        "-o", "--output",
        default="dependencies.csv",
        help="Path for the output CSV file (default: dependencies.csv)."
    )
    args = parser.parse_args()

    packages_path = Path(args.packages_dir)
    output_path = args.output

    if not packages_path.is_dir():
        print(f"Error: Directory '{packages_path}' not found. Please provide a valid path.")
        return

    print(f"🔍 Starting dependency scan... Output will be saved to '{output_path}'")
    
    with open(output_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["Package", "Dependency", "Version", "License", "Description"])

        for package_path in sorted(packages_path.iterdir()):
            if not package_path.is_dir():
                continue

            package_name = package_path.name
            print(f"\nProcessing package: {package_name}")

            if (package_path / "Cargo.toml").exists():
                fetch_rust_deps(package_path, package_name, writer)
            elif (package_path / "package.json").exists():
                fetch_npm_deps(package_path, package_name, writer)
            elif (package_path / "pom.xml").exists():
                fetch_maven_deps(package_path, package_name, writer)

    print(f"\n✅ Done! Dependency report saved to '{output_path}'.")

if __name__ == "__main__":
    main()