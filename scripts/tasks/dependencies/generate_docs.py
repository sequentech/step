#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import csv
import argparse
from pathlib import Path
from collections import defaultdict

# Package descriptions mapping
PACKAGE_DESCRIPTIONS = {
    'admin-portal': 'The admin portal is a React-based web application for administrative functions.',
    'b3': 'B3 is a Rust-based component providing cryptographic utilities and core functionality.',
    'ballot-verifier': 'The ballot verifier is a React-based application for verifying ballot integrity and authenticity.',
    'braid': 'Braid is a Rust-based component providing consensus and distributed systems functionality.',
    'e2e': 'E2E provides end-to-end testing capabilities and automation tools.',
    'ECIESEncryption': 'ECIESEncryption is a Java-based package providing elliptic curve encryption capabilities.',
    'electoral-log': 'Electoral Log provides comprehensive logging and auditing capabilities for electoral processes.',
    'harvest': 'Harvest provides data collection and processing capabilities.',
    'immu-board': 'Immu Board provides immutable board management and verification capabilities.',
    'immudb-rs': 'ImmuDB-RS provides Rust bindings for ImmuDB database operations.',
    'keycloak-extensions': 'Keycloak Extensions provides custom authentication and authorization extensions.',
    'orare': 'Orare provides procedural macro utilities and code generation capabilities.',
    'sequent-core': 'Sequent Core provides the fundamental libraries and utilities for the Sequent Voting Platform.',
    'step-cli': 'Step CLI provides command-line interface tools for managing the Sequent Voting Platform.',
    'strand': 'Strand provides cryptographic protocols and zero-knowledge proof capabilities.',
    'ui-core': 'UI Core provides shared user interface components and utilities.',
    'ui-essentials': 'UI Essentials provides essential user interface components and styling utilities.',
    'velvet': 'Velvet provides verification and validation tools for electoral processes.',
    'voting-portal': 'The voting portal is a React-based web application that provides the voter interface for elections.',
    'windmill': 'Windmill provides workflow automation and task orchestration capabilities.',
    'wrap-map-err': 'Wrap Map Err provides procedural macros for error handling utilities.',
}

# Package name formatting for display
def format_package_name(package_name):
    """Format package names for display in headers."""
    if package_name == 'ECIESEncryption':
        return 'ECIESEncryption'
    elif package_name == 'immudb-rs':
        return 'ImmuDB-RS'
    elif package_name in ['ui-core', 'ui-essentials']:
        return package_name.upper().replace('-', ' ')
    elif '-' in package_name:
        return ' '.join(word.capitalize() for word in package_name.split('-'))
    else:
        return package_name.capitalize()

def create_package_header_mapping():
    """Create mapping from CSV package names to markdown section headers."""
    mapping = {}
    for package_name in PACKAGE_DESCRIPTIONS.keys():
        formatted_name = format_package_name(package_name)
        mapping[package_name] = formatted_name
    return mapping

def parse_existing_markdown(markdown_path):
    """Parse existing markdown file and return sections with their content."""
    if not markdown_path.exists():
        return None, []
    
    with open(markdown_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    sections = []
    current_section = None
    current_content = []
    
    for i, line in enumerate(lines):
        if line.startswith('## ') and not line.startswith('## Overview') and not line.startswith('## License Compliance'):
            # Save previous section if it exists
            if current_section:
                sections.append({
                    'header': current_section,
                    'content_start': len(sections) > 0 and sections[-1]['content_end'] or 0,
                    'content_end': i,
                    'content': current_content[:]
                })
            
            # Start new section
            current_section = line.strip()
            current_content = []
        elif current_section:
            current_content.append(line)
    
    # Add the last section if it exists
    if current_section:
        sections.append({
            'header': current_section,
            'content_start': len(sections) > 0 and sections[-1]['content_end'] or 0,
            'content_end': len(lines),
            'content': current_content[:]
        })
    
    return lines, sections

def generate_dependency_table(dependencies):
    """Generate dependency table markdown for a package."""
    if not dependencies:
        return ''
    
    table_lines = []
    table_lines.append('| Dependency | Version | License | Description |')
    table_lines.append('|------------|---------|---------|-------------|')
    
    # Sort dependencies alphabetically
    sorted_deps = sorted(dependencies, key=lambda x: x['name'])
    
    for dep in sorted_deps:
        # Escape pipe characters in descriptions
        desc = dep['description'].replace('|', '\\|').replace('\n', ' ').strip()
        table_lines.append(f"| {dep['name']} | {dep['version']} | {dep['license']} | {desc} |")
    
    return '\n'.join(table_lines) + '\n'

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
    
    # Create package name to header mapping
    header_mapping = create_package_header_mapping()
    header_to_package = {v: k for k, v in header_mapping.items()}
    
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
            package_name = header_to_package.get(header_text)
            
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

def main():
    """Main function to parse arguments and generate documentation."""
    parser = argparse.ArgumentParser(
        description="Generate markdown documentation from dependencies CSV file."
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
        print(f"Error: Markdown file '{output_path}' not found.")
        return

    if not csv_path.exists():
        print(f"Error: CSV file '{csv_path}' not found.")
        return
    
    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    update_markdown_from_csv(csv_path, output_path)

if __name__ == "__main__":
    main()