#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import csv
import random
import string
import argparse

COL_NAME = 'password'

def generate_password(length=8):
    """
    Generates a random password of a specified length with a mix of
    uppercase letters, lowercase letters, and digits.
    """
    characters = string.ascii_letters + string.digits
    if length < 1:
        raise ValueError("Password length must be at least 1.")
    password = ''.join(random.choice(characters) for _ in range(length))
    return password

def add_pin_column_to_csv(input_filename, output_filename, password_length):
    """
    Reads a CSV file, adds a new column 'PIN' with a random password
    of a specified length, and writes the updated data to a new CSV file.
    """
    try:
        with open(input_filename, 'r', newline='', encoding='utf-8') as infile:
            reader = csv.reader(infile)
            header = next(reader)
            data = list(reader)

        # Add the new 'PIN' column to the header
        new_header = header + [COL_NAME]
        updated_data = []

        for row in data:
            # Generate a random password of the specified length and append it to the row
            pin = generate_password(password_length)
            row_with_pin = row + [pin]
            updated_data.append(row_with_pin)

        with open(output_filename, 'w', newline='', encoding='utf-8') as outfile:
            writer = csv.writer(outfile)
            writer.writerow(new_header)
            writer.writerows(updated_data)

        print(f"✅ Successfully added '{password_length}-character PIN' column and saved the new CSV to '{output_filename}'")
    
    except FileNotFoundError:
        print(f"❌ Error: The file '{input_filename}' was not found.")
    except Exception as e:
        print(f"❌ An error occurred: {e}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Adds a 'PIN' column with random passwords to a CSV file.")
    
    # Define positional arguments for input and output files
    parser.add_argument("input_file", help="The name of the input CSV file.")
    parser.add_argument("output_file", help="The name for the new output CSV file.")
    
    # Define the optional argument for password length
    parser.add_argument("-l", "--length", type=int, default=8,
                        help="The desired length of the password. Defaults to 8 characters.")
    
    args = parser.parse_args()
    
    add_pin_column_to_csv(args.input_file, args.output_file, args.length)