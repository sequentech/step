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

def generate_markdown_from_csv(csv_path, output_path):
    """Generate markdown documentation from CSV file."""
    
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
    
    # Sort packages alphabetically
    sorted_packages = sorted(packages.keys())
    
    # Generate markdown content
    markdown_content = '''---
id: third_party_deps
title: Third-Party Dependencies
---

# Third-Party Dependencies Reference

This document provides a comprehensive listing of all third-party dependencies used across the Sequent Voting Platform (SVP) packages, including their licenses and descriptions.

*The dependency information in this document is automatically generated using the `scripts/tasks/dependencies/list_deps.py` script, which scans package manifest files and queries package registries for metadata. For instructions on updating this information, see the README in the `scripts/tasks/dependencies/` directory.*

## Overview

The SVP monorepo contains packages written in multiple languages and using different package managers:

- **Rust packages**: Managed with Cargo, using dependencies from [crates.io](https://crates.io)
- **TypeScript/JavaScript packages**: Managed with npm/yarn, using dependencies from [npmjs.com](https://npmjs.com)
- **Java packages**: Managed with Maven, using dependencies from [Maven Central](https://central.sonatype.com)

Each package's dependencies are listed below with their version, license, and description information.

'''
    
    # Add each package section
    for package_name in sorted_packages:
        package_display_name = format_package_name(package_name)
        description = PACKAGE_DESCRIPTIONS.get(package_name, f'{package_display_name} package dependencies.')
        
        markdown_content += f'## {package_display_name}\n\n'
        markdown_content += f'{description}\n\n'
        markdown_content += '| Dependency | Version | License | Description |\n'
        markdown_content += '|------------|---------|---------|-------------|\n'
        
        # Sort dependencies alphabetically
        deps = sorted(packages[package_name], key=lambda x: x['name'])
        
        for dep in deps:
            # Escape pipe characters in descriptions
            desc = dep['description'].replace('|', '\\|').replace('\n', ' ').strip()
            markdown_content += f"| {dep['name']} | {dep['version']} | {dep['license']} | {desc} |\n"
        
        markdown_content += '\n'
    
    # Add footer
    markdown_content += '''---

## License Compliance

This documentation lists the licenses of all third-party dependencies for compliance and legal review purposes. Please ensure that all license requirements are met when distributing or deploying the Sequent Voting Platform.

For any questions about license compliance or dependency management, please consult the project maintainers or legal team.'''
    
    # Write to output file
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(markdown_content)
    
    print(f"✅ Generated markdown documentation: {output_path}")

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
    
    if not csv_path.exists():
        print(f"Error: CSV file '{csv_path}' not found.")
        return
    
    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    generate_markdown_from_csv(csv_path, output_path)

if __name__ == "__main__":
    main()