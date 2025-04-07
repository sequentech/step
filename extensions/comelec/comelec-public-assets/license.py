import os
import sys

def add_license_files(folder_path, extensions_to_process):
    """
    Traverses a folder and adds .license files to files with specified extensions.
    
    Args:
        folder_path: Path to the folder to traverse
        extensions_to_process: List of file extensions to process
    """
    # License text to add
    license_text = """SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
"""
    
    # Check if folder exists
    if not os.path.isdir(folder_path):
        print(f"Error: {folder_path} is not a valid directory")
        return
    
    # Walk through the folder
    for root, _, files in os.walk(folder_path):
        for file in files:
            # Check if the file has one of the specified extensions
            file_ext = os.path.splitext(file)[1].lower()
            if file_ext[1:] in extensions_to_process and not file.endswith('.license'):
                file_path = os.path.join(root, file)
                license_file_path = f"{file_path}.license"
                
                # Skip if license file already exists
                if os.path.exists(license_file_path):
                    print(f"License file already exists: {license_file_path}")
                    continue
                
                # Create license file
                try:
                    with open(license_file_path, 'w') as license_file:
                        license_file.write(license_text)
                    print(f"Created license file: {license_file_path}")
                except Exception as e:
                    print(f"Error creating license file for {file_path}: {e}")

if __name__ == "__main__":
    # Check if folder path is provided
    if len(sys.argv) < 2:
        print("Usage: python add_license_files.py <folder_path>")
        sys.exit(1)
    
    folder_path = sys.argv[1]
    
    # Extensions to process
    extensions_to_process = ['json', 'jpeg', 'jpg', 'pdf', 'svg']
    
    add_license_files(folder_path, extensions_to_process)