#!/usr/bin/env python3
import re
import sys

def remove_blob_binary(filename_in, filename_out):
    # Read the input file in binary mode
    with open(filename_in, 'rb') as f_in:
        content = f_in.read()

    # Define the pattern to match the _binary '...' blob data, handling escaped quotes
    # Since we're working with bytes, we need to use byte strings (prefix b'')
    pattern = re.compile(
        rb",_binary\s*'((?:[^'\\]|\\.|\\\n)*)'",  # Matches ,_binary '...'
        re.DOTALL  # Makes . match any character, including newlines
    )

    # Replace the blob data with ,'' (as bytes)
    content_cleaned = pattern.sub(b",''", content)

    # Write the output file in binary mode
    with open(filename_out, 'wb') as f_out:
        f_out.write(content_cleaned)

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: python remove_blob.py input.sql output.sql")
        sys.exit(1)

    input_filename = sys.argv[1]
    output_filename = sys.argv[2]

    remove_blob_binary(input_filename, output_filename)
