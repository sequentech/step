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

def render_sql():
    try:
        with open('templates/miru-sql.sql', 'r') as file:
            miru_template = file.read()
    except FileNotFoundError as e:
        logging.exception(f"Template file not found: {e}")
    except Exception as e:
        logging.exception("An error occurred while loading templates.")
        
    insert_boc_members ="""INSERT INTO `boc_members` VALUES ('chr1100', 'BOC Chairperson', '01', '1100','boc_chr1100', '2024-10-21 13:02:24');"""

    miru_context = {
        "boc_members": insert_boc_members
    }
    miru_sql = render_template(miru_template, miru_context)

    try:
        with open('data/miru.sql', 'w') as file:
            file.write(miru_sql)
        logging.info("data/miru.sql generated and saved successfully.")
    except Exception as e:
        logging.exception("An error occurred while saving data/miru.sql.")

render_sql()