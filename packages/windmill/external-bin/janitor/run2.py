# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
import json
import sys
import uuid
import time
from datetime import datetime, timezone
import sqlite3
import subprocess
import argparse
import os
import logging
from pybars import Compiler
import openpyxl
import re
import copy
import csv

def assert_folder_exists(folder_path):
    if not os.path.exists(folder_path):
        os.makedirs(folder_path)
        print(f"Created folder: {folder_path}")
    else:
        print(f"Folder already exists: {folder_path}")

# Step 0: ensure certain folders exist
assert_folder_exists("logs")
assert_folder_exists("data")
assert_folder_exists("output")

# Step 1: Set up logging
logging.basicConfig(
    filename='logs/process_log.log',  # Log file name
    level=logging.DEBUG,          # Log level
    format='%(asctime)s - %(levelname)s - %(message)s'
)

# Log the start of the script
logging.info("Script started.")

# Step 12: Compile and render templates using pybars3
compiler = Compiler()

def render_template(template_str, context):
    template = compiler.compile(template_str)
    return template(context)

table_format = {
    'boc_members': ['str', 'str', 'str', 'str', 'str', 'str'],
    'candidates': ['str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'int']
}

# Generate the VALUES part of the SQL statement
# We'll need to properly escape the values and format them as SQL literals
def sql_escape(value):
    return value.replace("'", "''")  # Escape single quotes by doubling them

def parse_table_values(file_path, table_name, table_format):
    # Read file as a CSV file with '|' delimiter
    try:
        import csv
        with open(file_path, 'r', newline='', encoding='utf-8') as csvfile:
            reader = csv.reader(csvfile, delimiter='|')
            rows = list(reader)
    except Exception as e:
        logging.exception(f"An error occurred while reading {file_path}.")
    
    table_len = len(table_format)
    
    rows_strs = []
    for row in rows:
        row_values = []
        row_len = len(row)
        for (i, format_element) in enumerate(table_format):
            if i >= row_len:
                row_values.append('')
            else:
                row_value = row[i]
                if 'int' == format_element:
                    row_value = str(int(row_value))
                else:
                    row_value = "'" + sql_escape(row_value) + "'"

                row_values.append(row_value)

        row_values_str = "(" + ", ".join(row_values) + ")"
        rows_strs.append(row_values_str)

    values_str =  ",".join(rows_strs)
    return f"INSERT INTO `{table_name}` VALUES {values_str};"

def render_sql(base_tables_path):
    try:
        with open('templates/miru-sql.sql', 'r') as file:
            miru_template = file.read()
    except FileNotFoundError as e:
        logging.exception(f"Template file not found: {e}")
    except Exception as e:
        logging.exception("An error occurred while loading templates.")
        
    insert_boc_members = parse_table_values(base_tables_path + 'BOCMembers.txt', 'boc_members', table_format['boc_members'] )
        
    insert_candidates = parse_table_values(base_tables_path + 'Candidates.txt', 'candidates', table_format['candidates'] )

    miru_context = {
        "boc_members": insert_boc_members,
        "candidates": insert_candidates
    }
    miru_sql = render_template(miru_template, miru_context)

    try:
        with open('data/miru.sql', 'w') as file:
            file.write(miru_sql)
        logging.info("data/miru.sql generated and saved successfully.")
    except Exception as e:
        logging.exception("An error occurred while saving data/miru.sql.")

render_sql('import-data/CCF-0-20241021/election_data/')