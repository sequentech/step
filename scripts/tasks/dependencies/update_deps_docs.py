#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import csv
import argparse
from pathlib import Path
from collections import defaultdict

# Mapping from markdown section headers to package directory names
HEADER_TO_PACKAGE = {
    'ECIESEncryption': 'ECIESEncryption',
    'Admin Portal': 'admin-portal',
    'B3': 'b3',
    'Ballot Verifier': 'ballot-verifier',
    'Braid': 'braid',
    'E2e': 'e2e',
    'Electoral Log': 'electoral-log',
    'Harvest': 'harvest',
    'Immu Board': 'immu-board',
    'ImmuDB-RS': 'immudb-rs',
    'Keycloak Extensions': 'keycloak-extensions',
    'Orare': 'orare',
    'Sequent Core': 'sequent-core',
    'Step Cli': 'step-cli',
    'Strand': 'strand',
    'UI CORE': 'ui-core',
    'UI ESSENTIALS': 'ui-essentials',
    'Velvet': 'velvet',
    'Voting Portal': 'voting-portal',
    'Windmill': 'windmill',
    'Wrap Map Err': 'wrap-map-err',
}

def update_markdown_from_csv(csv_path, output_path):
    """Update existing markdown documentation with CSV data, preserving structure."""
    
    # Read and organize dependencies by package
    packages = defaultdict(list)
    
    with open(csv_path, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            packages[row['Package']].append({
                'name': row['Dependency'],
                'version': row['Version'],
                'license': row['License'],
                'description': row['Description']
            })
    
    print(f"📊 Read {len(packages)} packages from CSV")

    # Read existing markdown file
    with open(output_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    print(f"🔍 Processing {len(lines)} lines in existing markdown")
    
    # Process file line by line and update tables
    updated_lines = []
    i = 0
    sections_updated = 0
    
    while i < len(lines):
        line = lines[i]
        updated_lines.append(line)
        
        # Check if this is a package section header
        if line.startswith('## ') and not line.startswith('## Overview') and not line.startswith('## License Compliance'):
            header_text = line.replace('## ', '').strip()
            package_name = HEADER_TO_PACKAGE.get(header_text)
            
            if package_name and package_name in packages:
                print(f"  -> Updating section '{header_text}' for package '{package_name}'")
                
                # Skip lines until we find the dependency table header or next section
                i += 1
                while i < len(lines) and not lines[i].strip().startswith('| Dependency | Version | License | Description |'):
                    if lines[i].startswith('## '):
                        # Hit next section, no table found
                        break
                    updated_lines.append(lines[i])
                    i += 1
                
                # If we found the table header
                if i < len(lines) and lines[i].strip().startswith('| Dependency | Version | License | Description |'):
                    # Add the table header and separator
                    updated_lines.append(lines[i])  # Header row
                    i += 1
                    if i < len(lines) and lines[i].strip().startswith('|----'):
                        updated_lines.append(lines[i])  # Separator row
                        i += 1
                    
                    # Skip existing table rows until next section or empty line followed by section
                    while i < len(lines):
                        if lines[i].startswith('## '):
                            break
                        elif lines[i].strip() == '' and i + 1 < len(lines) and lines[i + 1].startswith('## '):
                            break
                        elif not lines[i].strip().startswith('|') and lines[i].strip() != '':
                            break
                        i += 1
                    
                    # Generate and add new table rows
                    sorted_deps = sorted(packages[package_name], key=lambda x: x['name'])
                    for dep in sorted_deps:
                        desc = dep['description'].replace('|', '\\\\|').replace('\\n', ' ').strip()
                        updated_lines.append(f"| {dep['name']} | {dep['version']} | {dep['license']} | {desc} |\n")
                    
                    # Add blank line after table
                    if i < len(lines) and lines[i].strip() != '':
                        updated_lines.append('\n')
                    
                    sections_updated += 1
                    print(f"    -> Updated {len(packages[package_name])} dependencies")
                    
                    continue  # Don't increment i again at end of loop
            else:
                if package_name:
                    print(f"    -> Warning: Package '{package_name}' not found in CSV data")
        
        i += 1
    
    # Write updated content
    with open(output_path, 'w', encoding='utf-8') as f:
        f.writelines(updated_lines)
    
    print(f"✅ Updated {sections_updated} sections in markdown documentation: {output_path}")

def main():
    """Main function to parse arguments and generate documentation."""
    parser = argparse.ArgumentParser(
        description="Update markdown documentation with dependencies from CSV file."
    )
    parser.add_argument(
        "csv_file",
        help="Path to the dependencies CSV file."
    )
    parser.add_argument(
        "-o", "--output",
        required=True,
        help="Path for the output markdown file."
    )
    
    args = parser.parse_args()
    
    csv_path = Path(args.csv_file)
    output_path = Path(args.output)
    
    if not output_path.exists():
        print(f"Error: Markdown file '{output_path}' not found. This script only updates existing files.")
        return

    if not csv_path.exists():
        print(f"Error: CSV file '{csv_path}' not found.")
        return
    
    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    update_markdown_from_csv(csv_path, output_path)

if __name__ == "__main__":
    main()