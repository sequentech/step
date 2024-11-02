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

def remove_file_if_exists(file_path):
    if os.path.exists(file_path):
        os.remove(file_path)
        print(f"Removed file: {file_path}")
    else:
        print(f"File does not exist: {file_path}")

# Step 12: Compile and render templates using pybars3
compiler = Compiler()

def render_template(template_str, context):
    template = compiler.compile(template_str)
    return template(context)

table_format = {
    'boc_members': ['str', 'str', 'str', 'str', 'str', 'str'],
    'candidates': ['str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'int'],
    'ccs': ['str', 'str', 'str', 'str', 'str', 'str', 'str', 'str'],
    'contest': ['str', 'str', 'str', 'str', 'str', 'str', 'str'],
    'eb_members': ['str', 'str', 'str', 'str', 'str', 'str'],
    'political_organizations': ['str', 'str', 'str', 'str'],
    'polling_centers': ['str', 'str', 'str', 'str', 'int', 'str', 'str', 'str'],
    'polling_district_region': ['str', 'str', 'str'],
    'polling_district': ['str', 'str', 'str', 'int', 'str'],
    'precinct_established': ['str', 'str', 'str', 'int', 'str', 'str'],
    'precinct': ['str', 'str', 'str', 'str', 'int', 'str', 'str', 'str'],
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
    
    rows_strs = []
    for row in rows:
        row_values = []
        row_len = len(row)
        for (i, format_element) in enumerate(table_format):
            if i >= row_len:
                row_values.append('NULL')
            else:
                row_value = row[i]
                if 'NULL' == row_value:
                    row_values.append('NULL')
                else:
                    if 'int' == format_element:
                        row_value = str(int(row_value))
                    else:
                        row_value = "'" + sql_escape(row_value) + "'"

                    row_values.append(row_value)

        row_values_str = "(" + ", ".join(row_values) + ")"
        rows_strs.append(row_values_str)

    values_str =  ",".join(rows_strs)
    return f"INSERT INTO `{table_name}` VALUES {values_str};"

def render_sql(base_tables_path, output_path):
    try:
        with open('templates/miru-sql.sql', 'r') as file:
            miru_template = file.read()
    except FileNotFoundError as e:
        logging.exception(f"Template file not found: {e}")
    except Exception as e:
        logging.exception("An error occurred while loading templates.")
        
    boc_members = parse_table_values(base_tables_path + 'BOCMembers.txt', 'boc_members', table_format['boc_members'] )
    candidates = parse_table_values(base_tables_path + 'Candidates.txt', 'candidates', table_format['candidates'] )
    ccs = parse_table_values(base_tables_path + 'CCS.txt', 'ccs', table_format['ccs'] )
    contest = parse_table_values(base_tables_path + 'Contest.txt', 'contest', table_format['contest'] )
    eb_members = parse_table_values(base_tables_path + 'EBMembers.txt', 'eb_members', table_format['eb_members'] )
    political_organizations = parse_table_values(base_tables_path + 'Political_Organizations.txt', 'political_organizations', table_format['political_organizations'] )
    polling_centers = parse_table_values(base_tables_path + 'Polling_Centers.txt', 'polling_centers', table_format['polling_centers'] )
    polling_district_region = parse_table_values(base_tables_path + 'Polling_District_Region.txt', 'polling_district_region', table_format['polling_district_region'] )
    polling_district = parse_table_values(base_tables_path + 'Polling_District.txt', 'polling_district', table_format['polling_district'] )
    precinct_established = parse_table_values(base_tables_path + 'Precinct_Established.txt', 'precinct_established', table_format['precinct_established'] )
    precinct = parse_table_values(base_tables_path + 'Precinct.txt', 'precinct', table_format['precinct'] )


    miru_context = {
        "boc_members": boc_members,
        "candidates": candidates,
        "ccs": ccs,
        "contest": contest,
        "eb_members": eb_members,
        "political_organizations": political_organizations,
        "polling_centers": polling_centers,
        "polling_district_region": polling_district_region,
        "polling_district": polling_district,
        "precinct_established": precinct_established,
        "precinct": precinct
    }
    miru_sql = render_template(miru_template, miru_context)

    try:
        with open(output_path, 'w') as file:
            file.write(miru_sql)
        logging.info("data/miru.sql generated and saved successfully.")
    except Exception as e:
        logging.exception("An error occurred while saving data/miru.sql.")

def run_command(command, script_dir):
    # Run the command using subprocess.run() with shell=True
    try:
        result = subprocess.run(command, cwd=script_dir, shell=True, capture_output=True, text=True)
        if result.returncode == 0:
            logging.info("Command ran successfully.")
            logging.debug(f"Command output: {result.stdout}")
        else:
            logging.error("Command failed.")
            logging.error(f"Error: {result.stderr}")
    except Exception as e:
        logging.exception("An error occurred while running the command.")


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

# Step 2: Convert the csv to sql
sql_output_path = 'data/miru.sql'
sqlite_output_path = 'data/db_sqlite_miru.db'
remove_file_if_exists(sql_output_path)
remove_file_if_exists(sqlite_output_path)
render_sql('import-data/CCF-0-20241021/election_data/', sql_output_path)

# Determine the script's directory to use as cwd
script_dir = os.path.dirname(os.path.abspath(__file__))

# Step 3: Convert MySQL dump to SQLite
command = f"chmod +x mysql2sqlite && ./mysql2sqlite {sql_output_path} | sqlite3 {sqlite_output_path}"

# Log the constructed command
logging.debug(f"Constructed command: {command}")

run_command(command, script_dir)