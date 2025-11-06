#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""
Update SPDX license headers in source files.

This script:
1. Finds files with "SPDX" and "sequentech.io>" in the first 10 lines
2. Replaces those lines with the current year copyright notice
3. Removes any duplicate SPDX-FileCopyrightText lines
4. Only processes text files
5. Respects REUSE.toml exclusions
6. Processes files in streaming mode
"""

import os
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional, Iterator, Set
import mimetypes


def get_current_year() -> str:
    """Get the current year as a string."""
    return str(datetime.now().year)


def is_text_file(file_path: str) -> bool:
    """
    Check if a file is a text file by analyzing its content.
    
    Similar to the `file` command, this function reads the beginning of the file
    and checks for binary indicators and text characteristics.

    Args:
        file_path: Path to the file

    Returns:
        True if the file appears to be a text file
    """
    try:
        with open(file_path, 'rb') as f:
            # Read a reasonable chunk for analysis (similar to `file` command)
            chunk = f.read(8192)
            
            if len(chunk) == 0:
                # Empty files are considered text
                return True
            
            # Check for null bytes (strong indicator of binary)
            if b'\x00' in chunk:
                return False
            
            # Count non-text bytes
            # Text files should have mostly printable characters, whitespace, and common control chars
            text_chars = bytearray({7, 8, 9, 10, 12, 13, 27} | set(range(0x20, 0x100)))
            non_text_count = sum(1 for byte in chunk if byte not in text_chars)
            
            # If more than 30% non-text characters, likely binary
            if non_text_count / len(chunk) > 0.30:
                return False
            
            # Try to decode as UTF-8 (most common text encoding)
            try:
                chunk.decode('utf-8')
                return True
            except UnicodeDecodeError:
                # Try other common encodings
                for encoding in ['latin-1', 'iso-8859-1', 'cp1252']:
                    try:
                        chunk.decode(encoding)
                        return True
                    except (UnicodeDecodeError, LookupError):
                        continue
                
                # If we can't decode it, it's likely binary
                return False
                
    except (OSError, PermissionError):
        return False


def parse_reuse_toml(root_dir: str) -> Set[str]:
    """
    Parse REUSE.toml and extract excluded paths.

    Args:
        root_dir: Root directory of the project

    Returns:
        Set of glob patterns to exclude
    """
    reuse_file = Path(root_dir) / 'REUSE.toml'
    if not reuse_file.exists():
        return set()

    excluded_patterns = set()
    try:
        import tomli
    except ImportError:
        try:
            import tomllib as tomli
        except ImportError:
            print("Warning: tomli/tomllib not available, cannot parse REUSE.toml", file=sys.stderr)
            return set()

    try:
        with open(reuse_file, 'rb') as f:
            data = tomli.load(f)

        # Extract paths from annotations
        for annotation in data.get('annotations', []):
            paths = annotation.get('path', [])
            if isinstance(paths, str):
                excluded_patterns.add(paths)
            elif isinstance(paths, list):
                excluded_patterns.update(paths)

    except Exception as e:
        print(f"Warning: Error parsing REUSE.toml: {e}", file=sys.stderr)

    return excluded_patterns


def should_exclude_path(file_path: str, root_dir: str, excluded_patterns: Set[str]) -> bool:
    """
    Check if a file path matches any exclusion pattern.

    Args:
        file_path: Path to check
        root_dir: Root directory
        excluded_patterns: Set of glob patterns to exclude

    Returns:
        True if the path should be excluded
    """
    try:
        rel_path = Path(file_path).relative_to(root_dir)
    except ValueError:
        # File is not relative to root_dir
        return False

    rel_path_str = str(rel_path)

    for pattern in excluded_patterns:
        # Simple glob pattern matching
        if '**' in pattern:
            # Recursive pattern like "hasura/**" or "**/*.xlsx"
            pattern_parts = pattern.split('**')
            if len(pattern_parts) == 2:
                prefix, suffix = pattern_parts
                prefix = prefix.rstrip('/')
                suffix = suffix.lstrip('/')

                # Check prefix
                if prefix and not rel_path_str.startswith(prefix):
                    continue

                # Check suffix - need to handle both directory and file patterns
                if suffix:
                    # For patterns like "**/*.xlsx", check if any part of path matches
                    if suffix.startswith('*'):
                        # File pattern like "/*.xlsx"
                        import fnmatch
                        if fnmatch.fnmatch(rel_path_str, '*' + suffix):
                            return True
                        # Pattern didn't match, continue to next pattern
                        continue
                    else:
                        # Directory/file pattern
                        if not rel_path_str.endswith(suffix):
                            continue

                # If we get here, prefix matched and suffix matched (or was empty)
                return True
        elif '*' in pattern:
            # Simple wildcard
            import fnmatch
            if fnmatch.fnmatch(rel_path_str, pattern):
                return True
        else:
            # Exact match or directory match
            if rel_path_str == pattern or rel_path_str.startswith(pattern + '/'):
                return True

    return False


def detect_comment_style(line: str) -> Optional[str]:
    """
    Detect the comment style from a line.

    Returns the comment prefix (e.g., '//', '#', '/*', etc.)
    """
    line = line.lstrip()
    # Handle potential multiple comment prefixes (e.g., "// //")
    if line.startswith('//'):
        return '//'
    elif line.startswith('#'):
        return '#'
    elif line.startswith('/*'):
        return '/*'
    elif line.startswith('*'):
        return '*'
    elif line.startswith('--'):
        return '--'
    return None


def update_license_header(
    file_path: str,
    dry_run: bool = False,
    header_template: str = "{comment_prefix} SPDX-FileCopyrightText: {year} Sequent Tech Inc <legal@sequentech.io>\n",
    header_pattern: str = r"SPDX.*sequentech\.io"
) -> bool:
    """
    Update the license header in a file.

    Args:
        file_path: Path to the file to update
        dry_run: If True, only report changes without modifying the file
        header_template: Template for the new header line. Available placeholders:
                        {comment_prefix}, {year}
        header_pattern: Regex pattern to match the line to replace

    Returns:
        True if the file was modified (or would be modified in dry-run mode)
    """
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except UnicodeDecodeError as e:
        print(f"Error: Cannot decode {file_path}: {e}", file=sys.stderr)
        return False
    except PermissionError as e:
        print(f"Error: Permission denied {file_path}: {e}", file=sys.stderr)
        return False
    except Exception as e:
        print(f"Error: Cannot read {file_path}: {e}", file=sys.stderr)
        return False

    if len(lines) == 0:
        return False

    # Process the first 10 lines
    header_lines = lines[:min(10, len(lines))]
    body_lines = lines[min(10, len(lines)):]

    # Find the line matching the header pattern
    target_line_idx = None
    comment_prefix = None
    pattern = re.compile(header_pattern, re.IGNORECASE)

    for idx, line in enumerate(header_lines):
        if pattern.search(line):
            target_line_idx = idx
            comment_prefix = detect_comment_style(line)
            break

    # If no matching line found, return early
    if target_line_idx is None:
        return False

    if comment_prefix is None:
        print(f"Warning: Could not detect comment style in {file_path}", file=sys.stderr)
        return False

    # Create the new header line
    current_year = get_current_year()
    new_header = header_template.format(comment_prefix=comment_prefix, year=current_year)
    if not new_header.endswith('\n'):
        new_header += '\n'

    # Check if the line is already up to date
    if header_lines[target_line_idx].strip() == new_header.strip():
        return False

    # Build the new header
    new_header_lines = []
    replaced = False

    for idx, line in enumerate(header_lines):
        if idx == target_line_idx:
            # Replace the target line
            new_header_lines.append(new_header)
            replaced = True
        elif 'SPDX-FileCopyrightText' in line and replaced:
            # Skip duplicate SPDX-FileCopyrightText lines after replacement
            continue
        else:
            new_header_lines.append(line)

    # Reconstruct the file content
    new_content = new_header_lines + body_lines

    if dry_run:
        print(f"Would update: {file_path}")
        print(f"  Old: {header_lines[target_line_idx].strip()}")
        print(f"  New: {new_header.strip()}")
        return True

    # Write the updated content
    try:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(new_content)
        print(f"Updated: {file_path}")
        return True
    except PermissionError as e:
        print(f"Error: Cannot write to {file_path}: {e}", file=sys.stderr)
        return False
    except Exception as e:
        print(f"Error: Failed to update {file_path}: {e}", file=sys.stderr)
        return False


def walk_files(root_dir: str, patterns: list[str], excluded_patterns: Set[str]) -> Iterator[str]:
    """
    Walk directory tree and yield matching files in streaming mode.

    Args:
        root_dir: Root directory to search
        patterns: List of glob patterns to match. If None or empty, matches all text files.
        excluded_patterns: Set of patterns to exclude

    Yields:
        File paths that match the patterns
    """
    root_path = Path(root_dir).resolve()

    # Common directories to skip
    skip_dirs = {
        'node_modules', '.git', 'dist', 'build', 'target',
        '.vscode', '.idea', '__pycache__', '.cache',
        'coverage', '.pytest_cache', '.mypy_cache'
    }

    for dirpath, dirnames, filenames in os.walk(root_path):
        # Remove skip_dirs from dirnames to prevent walking into them
        dirnames[:] = [d for d in dirnames if d not in skip_dirs]

        for filename in filenames:
            file_path = Path(dirpath) / filename
            file_path_str = str(file_path)

            # Check exclusion patterns
            if should_exclude_path(file_path_str, str(root_path), excluded_patterns):
                continue

            # Check if file matches any pattern (if patterns provided)
            matches_pattern = not patterns  # If no patterns, match all text files
            if patterns:
                for pattern in patterns:
                    # Simple pattern matching
                    if pattern.startswith('**/*.'):
                        # Match by extension
                        ext = pattern[4:]  # Remove '**/.'
                        if filename.endswith(ext):
                            matches_pattern = True
                            break
                    elif pattern.startswith('*.'):
                        # Match by extension
                        ext = pattern[1:]  # Remove '*'
                        if filename.endswith(ext):
                            matches_pattern = True
                            break
                    elif pattern.startswith('**/'):
                        # Match filename pattern after **/
                        import fnmatch
                        pattern_suffix = pattern[3:]  # Remove '**/'
                        if fnmatch.fnmatch(filename, pattern_suffix):
                            matches_pattern = True
                            break

            if matches_pattern:
                # Check if it's a text file
                if is_text_file(file_path_str):
                    yield file_path_str


def main():
    """Main entry point."""
    import argparse

    parser = argparse.ArgumentParser(
        description='Update SPDX license headers in source files'
    )
    parser.add_argument(
        'paths',
        nargs='*',
        default=['.'],
        help='Files or directories to process (default: current directory)'
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Show what would be changed without modifying files'
    )
    parser.add_argument(
        '--pattern',
        action='append',
        help='File patterns to search (can be specified multiple times)'
    )
    parser.add_argument(
        '--header-template',
        default='{comment_prefix} SPDX-FileCopyrightText: {year} Sequent Tech Inc <legal@sequentech.io>',
        help='Template for the header line. Available placeholders: {comment_prefix}, {year}. '
             'Default: "{comment_prefix} SPDX-FileCopyrightText: {year} Sequent Tech Inc <legal@sequentech.io>"'
    )
    parser.add_argument(
        '--header-pattern',
        default=r'SPDX.*sequentech\.io',
        help='Regex pattern to match the line to replace. '
             'Default: "SPDX.*sequentech\\.io"'
    )

    args = parser.parse_args()

    # If no patterns specified, process all text files
    # Otherwise use the patterns provided by the user
    patterns = args.pattern if args.pattern else []

    # Process each path argument
    modified_count = 0
    error_count = 0

    for path_arg in args.paths:
        path = Path(path_arg).resolve()

        if not path.exists():
            print(f"Warning: {path_arg} does not exist", file=sys.stderr)
            continue

        # Find root directory (where REUSE.toml is)
        root_dir = path if path.is_dir() else path.parent
        while root_dir.parent != root_dir:
            if (root_dir / 'REUSE.toml').exists():
                break
            root_dir = root_dir.parent

        # Parse REUSE.toml exclusions
        excluded_patterns = parse_reuse_toml(str(root_dir))

        if path.is_file():
            # Process single file
            if is_text_file(str(path)):
                if not should_exclude_path(str(path), str(root_dir), excluded_patterns):
                    try:
                        if update_license_header(str(path), dry_run=args.dry_run, header_template=args.header_template, header_pattern=args.header_pattern):
                            modified_count += 1
                    except Exception as e:
                        print(f"Error processing {path}: {e}", file=sys.stderr)
                        error_count += 1
        elif path.is_dir():
            # Process directory in streaming mode
            for file_path in walk_files(str(path), patterns, excluded_patterns):
                try:
                    if update_license_header(file_path, dry_run=args.dry_run, header_template=args.header_template, header_pattern=args.header_pattern):
                        modified_count += 1
                except Exception as e:
                    print(f"Error processing {file_path}: {e}", file=sys.stderr)
                    error_count += 1
        else:
            print(f"Warning: {path_arg} is not a valid file or directory", file=sys.stderr)

    # Print summary
    action = "Would update" if args.dry_run else "Updated"
    print(f"\n{action} {modified_count} file(s)")
    if error_count > 0:
        print(f"Encountered {error_count} error(s)", file=sys.stderr)

    return 0 if error_count == 0 else 1


if __name__ == '__main__':
    sys.exit(main())
