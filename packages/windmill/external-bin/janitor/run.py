# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
import json
import sys
import uuid
from datetime import datetime, timezone
import sqlite3
import subprocess
import argparse
import os
import logging
from pybars import Compiler
import openpyxl
import copy
import csv
import zipfile
import io
import shutil
import hashlib
import pyzipper
from pathlib import Path
from patch import parse_table_sheet, parse_parameters, patch_json_with_excel
import re

IS_DEBUG = False

def is_valid_regex(pattern):
    try:
        re.compile(pattern)  # Try to compile the regex
        return True          # If successful, it's a valid regex
    except:
        return False         # If re.error is raised, it's not a valid regex

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

def remove_folder_if_exists(folder_path):
    if not os.path.exists(folder_path):
        print(f"Folder does not exist: {folder_path}")
        return
    if not os.path.isdir(folder_path):
        print(f"Path is not a folder {folder_path}")
        return

    shutil.rmtree(folder_path)
    print(f"Removed folder and its contents: {folder_path}")

# Step 12: Compile and render templates using pybars3
compiler = Compiler()

def render_template(template_str, context):
    template = compiler.compile(template_str)
    return template(context)

table_format = {
    'boc_members': ['str', 'str', 'str', 'str', 'str', 'str'],
    'candidates': ['str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'int', 'str', 'str'],
    'ccs': ['str', 'str', 'str', 'str', 'str', 'str', 'str', 'str', 'str'],
    'contest': ['str', 'str', 'str', 'str', 'str', 'str', 'str'],
    'contest_class': ['str', 'str', 'str', 'str', 'int', 'str', 'str'],
    'eb_members': ['str', 'str', 'str', 'str', 'str', 'str'],
    'political_organizations': ['str', 'str', 'str', 'str'],
    'polling_centers': ['str', 'str', 'str', 'str', 'int', 'str', 'str', 'str'],
    'polling_district_region': ['str', 'str', 'str'],
    'polling_district': ['str', 'str', 'str', 'int', 'str'],
    'precinct_established': ['str', 'str', 'str', 'int', 'str', 'str'],
    'precinct': ['str', 'str', 'str', 'str', 'int', 'str', 'str', 'str'],
    'region': ['str', 'str', 'str', 'str', 'str'],
    'voting_device': ['str', 'str', 'str', 'str', 'str', 'str'],
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

def render_sql(base_tables_path, output_path, voters_table_path):
    try:
        with open('templates/miru-sql.sql', 'r') as file:
            miru_template = file.read()
    except FileNotFoundError as e:
        logging.exception(f"Template file not found: {e}")
    except Exception as e:
        logging.exception("An error occurred while loading templates.")
    
    try:
        if voters_table_path:
            with open(voters_table_path, 'r') as file:
                voters_table = file.read()
        else:
            voters_table = ''
    except FileNotFoundError as e:
        logging.exception(f"Voters table file not found: {e}")
        
    candidates = parse_table_values(os.path.join(base_tables_path, 'Candidates.txt'), 'candidates', table_format['candidates'] )
    contest = parse_table_values(os.path.join(base_tables_path, 'Contest.txt'), 'contest', table_format['contest'] )
    contest_class = parse_table_values(os.path.join(base_tables_path, 'Contest_Class.txt'), 'contest_class', table_format['contest_class'] )
    polling_centers = parse_table_values(os.path.join(base_tables_path, 'Polling_Centers.txt'), 'polling_centers', table_format['polling_centers'] )
    polling_district_region = parse_table_values(os.path.join(base_tables_path, 'Polling_District_Region.txt'), 'polling_district_region', table_format['polling_district_region'] )
    polling_district = parse_table_values(os.path.join(base_tables_path, 'Polling_District.txt'), 'polling_district', table_format['polling_district'] )
    precinct_established = parse_table_values(os.path.join(base_tables_path, 'Precinct_Established.txt'), 'precinct_established', table_format['precinct_established'] )
    precinct = parse_table_values(os.path.join(base_tables_path, 'Precinct.txt'), 'precinct', table_format['precinct'] )
    region = parse_table_values(os.path.join(base_tables_path, 'Region.txt'), 'region', table_format['region'] )

    miru_context = {
        "candidates": candidates,
        "contest": contest,
        "contest_class": contest_class,
        "polling_centers": polling_centers,
        "polling_district_region": polling_district_region,
        "polling_district": polling_district,
        "precinct_established": precinct_established,
        "precinct": precinct,
        "region": region,
        "voters_table": voters_table
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
        logging.info(f"Running command: {command}")
        result = subprocess.run(command, cwd=script_dir, shell=True, capture_output=True, text=True)
        if result.returncode == 0:
            logging.info("Command ran successfully.")
            logging.debug(f"Command output: {result.stdout}")
            return result.stdout
        else:
            print(f"Running command: {command}")
            print("Command failed.")
            print(f"Error: {result}")
            raise Exception(result)
    except Exception as e:
        logging.exception("An error occurred while running the command.")
        raise e


# Step 11: Retrieve data from SQLite database
def get_sqlite_data(query, dbfile):
    try:
        conn = sqlite3.connect(dbfile)
        conn.row_factory = sqlite3.Row  # This allows rows to be accessed like dictionaries
        cursor = conn.cursor()
        logging.info(f"Connected to SQLite database at {dbfile}.")
    except sqlite3.Error as e:
        logging.exception(f"Failed to connect to SQLite database: {e}")
        
    try:
        cursor.execute(query)
        result = cursor.fetchall()
        logging.debug(f"Query executed: {query}, Result: {result}")
    except sqlite3.Error as e:
        logging.exception(f"Failed to execute query: {query}")
        raise e
    
    # Close the SQLite connection
    try:
        conn.close()
        logging.info("SQLite connection closed.")
    except sqlite3.Error as e:
        logging.exception(f"Failed to close SQLite connection: {e}")

    return result

def get_voters(sqlite_output_path):
    query = """SELECT 
        voter_demo_ovcs.FIRSTNAME as voter_FIRSTNAME,
        voter_demo_ovcs.LASTNAME as voter_LASTNAME,
        voter_demo_ovcs.MATERNALNAME as voter_MATERNALNAME,
        voter_demo_ovcs.DATEOFBIRTH as voter_DATEOFBIRTH,
        voter_demo_ovcs.COUNTRY as voter_COUNTRY
    FROM
        voter_demo_ovcs;
    """
    return get_sqlite_data(query, sqlite_output_path)

def get_data(sqlite_output_path, excel_data):
    precinct_ids = [e["precinct_id"] for e in excel_data["areas"]]
    precinct_ids_str = ",".join([f"'{precinct_id}'" for precinct_id in precinct_ids])

    query = f"""SELECT
        region.REGION_CODE as pop_POLLCENTER_CODE,
        precinct.PRECINCT_CODE as DB_TRANS_SOURCE_ID,
        precinct_established.ESTABLISHED_CODE as DB_PRECINCT_ESTABLISHED_CODE,
        polling_centers.VOTING_CENTER_CODE as allbgy_ID_BARANGAY,
        polling_centers.VOTING_CENTER_NAME as allbgy_AREANAME,
        polling_centers.VOTING_CENTER_ADDR  as DB_ALLMUN_AREA_NAME,
        region.REGION_NAME as DB_POLLING_CENTER_POLLING_PLACE,
        polling_district.DESCRIPTION as DB_CONTEST_NAME,
        polling_district.POLLING_DISTRICT_NUMBER as DB_RACE_ELIGIBLEAMOUNT,
        polling_district.POLLING_DISTRICT_CODE as DB_SEAT_DISTRICTCODE,
        contest_class.PRECEDENCE as contest_SORT_ORDER,
        candidates.CANDIDATE_CODE as DB_CANDIDATE_CAN_CODE,
        candidates.NAME_ON_BALLOT as DB_CANDIDATE_NAMEONBALLOT,
        candidates.MANUAL_ORDER as DB_CANDIDATE_SORT_ORDER,
        candidates.POLITICAL_ORG_CODE as DB_CANDIDATE_NOMINATEDBY
    FROM
        precinct
    JOIN
        precinct_established
    ON
        precinct_established.ESTABLISHED_CODE = precinct.ESTABLISHED_CODE AND
        precinct_established.PRECINCT_CODE = precinct.PRECINCT_CODE
    JOIN
        region
    ON
        region.REGION_CODE = precinct_established.REGION_CODE
    JOIN
        polling_centers
    ON
        region.REGION_CODE = polling_centers.REGION_CODE
    CROSS JOIN
        polling_district
    JOIN
        contest
    ON
        contest.CONTEST_CODE = polling_district.POLLING_DISTRICT_CODE
    JOIN
        contest_class
    ON
        contest_class.CONTEST_CLASS_CODE = contest.CONTEST_CLASS_CODE
    JOIN
        candidates
    ON
        candidates.CONTEST_CODE = contest.CONTEST_CODE
    WHERE
        precinct.PRECINCT_CODE IN ({precinct_ids_str}) AND
        polling_district.POLLING_DISTRICT_NAME = 'PHILIPPINES';
    """
    print(query)
    return get_sqlite_data(query, sqlite_output_path)

def read_base_config():
    try:
        with open('config/baseConfig.json', 'r') as file:
            base_config = json.load(file)
            logging.info("Loaded base configuration.")
            return base_config
    except FileNotFoundError:
        logging.exception("Base configuration file not found.")
    except json.JSONDecodeError:
        logging.exception("Failed to parse base configuration file.")

def generate_uuid():
    return str(uuid.uuid4())
logging.debug(f"Generated UUID: {generate_uuid()}")

def get_sbei_username(user, region_code):
    return f"sbei-{region_code}-{user['ROLE']}"

def get_trustee_username(user, region_code):
    return f"trustee-{region_code}-{user['ROLE']}"  

def generate_election_event(excel_data, base_context, miru_data, results):
    election_event_id = generate_uuid()
    miru_event = list(miru_data.values())[0]

    sbei_users = []
    sbei_users_with_permission_labels = []
    precinct_to_region = {}
    region_to_precincts = {}

    excel_areas_by_precinct_id = set(str(area["precinct_id"]) for area in excel_data["areas"])

    for row in results:
        precinct_id = row["DB_TRANS_SOURCE_ID"]

        area_context = next((area for area in excel_data["areas"] if str(area["precinct_id"]) == precinct_id), None)

        region_code = str(area_context["region_code_overwrite"] if "region_code_overwrite" in area_context and area_context["region_code_overwrite"] is not None else row["pop_POLLCENTER_CODE"])

        if precinct_id not in precinct_to_region:
            precinct_to_region[precinct_id] = region_code
        
        if region_code not in region_to_precincts:
            region_to_precincts[region_code] = set()
        region_to_precincts[region_code].add(precinct_id)


    trustees_by_region = {}
    perm_labels_by_region = {}
    for precinct_id in miru_data.keys():
        if precinct_id not in excel_areas_by_precinct_id:
            continue
        region_code = precinct_to_region[precinct_id]

        if region_code not in trustees_by_region:
            region_precincts = region_to_precincts[region_code]
            excel_election = next((e for e in excel_data["elections"] if str(e["precinct_id"]) in region_precincts), None)
            if excel_election is not None:
                election_trustees = excel_election["trustees"].split("|")
                trustees_by_region[region_code] = election_trustees

                election_permission_label = excel_election["permission_label"]
                if election_permission_label:
                    if region_code not in perm_labels_by_region:
                        perm_labels_by_region[region_code] = set()
                    perm_labels_by_region[region_code].add(election_permission_label)


    for precinct_id in miru_data.keys():
        if precinct_id not in excel_areas_by_precinct_id:
            continue
        precinct = miru_data[precinct_id]
        miru_election_id = "1"

        if precinct_id not in precinct_to_region:
            raise Exception(f"precinct with 'id' = {precinct_id} not found in precinct_to_region")

        region_code = precinct_to_region[precinct_id]
        if region_code not in trustees_by_region:
            raise Exception(f"trustees not found for region = {region_code} and 'precinct_id' = {precinct_id}")
        election_trustees = trustees_by_region[region_code]

        region_perm_labels = list(perm_labels_by_region.get(region_code, set()))
        
        for user in precinct["USERS"]:
            base_user = {
            "miru_id": user["ID"],
            "miru_role": user["ROLE"],
            "miru_name": user["NAME"],
            "miru_election_id": miru_election_id
            }
            for get_username in [get_sbei_username, get_trustee_username]:
                new_user = copy.deepcopy(base_user)
                new_user["username"] = get_username(user, region_code)
                sbei_users.append(new_user)
                is_trustee = get_username == get_trustee_username
                add_perm_label = "OFOV" if is_trustee else "SBEI"
                user_perm_labels = region_perm_labels.copy()
                user_perm_labels.append(add_perm_label)

                perm_labels = list(set(user_perm_labels))

                trustee_id = ""
                if is_trustee and election_trustees:
                    role_idx = int(user["ROLE"]) - 1
                    trustee_id = election_trustees[role_idx]

                sbei_users_with_permission_labels.append({
                    "permission_label": perm_labels,
                    "username": new_user["username"],
                    "miru_id": user["ID"],
                    "miru_role": user["ROLE"],
                    "miru_name": user["NAME"],
                    "miru_election_id": miru_election_id,
                    "trustee": "trustee" if is_trustee else "",
                    "trustee_id": trustee_id
                })

    sbei_users_str = json.dumps(sbei_users)
    sbei_users_str = sbei_users_str.replace('"', '\\"')
    election_event_context = {
        "UUID": election_event_id,
        "miru": {
            "event_id": miru_event["EVENT_ID"],
            "event_name": miru_event["EVENT_NAME"],
            "sbei_users": sbei_users_str
        },
        **base_context,
        **excel_data["election_event"]

    }
    #print(election_event_context)
    temp_render = render_template(election_event_template, election_event_context)
    return json.loads(temp_render), election_event_id, sbei_users_with_permission_labels


# "OSAKA PCG" -> "Osaka PCG"
# WASHINGTON D.C. PE -> Washington D.C .PE
# ISLE OF MAN -> Isle Of Man
# HOLY SEE -> Holy See
# NEW YORK PGC -> New York PGC
def get_embassy(embassy):
    # Split the input string into words
    without_parentheses = re.sub(r"\(.*?\)", "", embassy)
    words = without_parentheses.split()

    special_words = ["D.C.", "DC", "SAR", "ROC"]
    
    # Capitalize each word, and handle the last word conditionally
    formatted_words = [word.title() if word.upper() not in special_words else word.upper()  for word in words[:-1]]
    last_word = words[-1].upper() if len(words[-1]) <= 3 else words[-1].title()
    
    # Combine the formatted words with the conditionally formatted last word
    formatted_words.append(last_word)
    
    # Join the words into a single string
    return " ".join(formatted_words)

def get_country_from_area_embassy(area, embassy):
    country = get_embassy(area)
    return f"{country}/{embassy}"

def generate_reports_csv(reports, election_event_id):
    reports_array = [
        {
            "ID": report["id"],
            "Election ID": report["election_id"],
            "Report Type": report["report_type"],
            "Template Alias": report["template_alias"],
            "Cron Config": json.dumps(report.get("cron_config", None)),
            "Encryption Policy": report["encryption_policy"],
            "Password": report["password"],
            "Permission Labels": report["permission_label"]
        } for report in reports
    ]

    csv_buffer = io.StringIO()

    writer = csv.DictWriter(csv_buffer, fieldnames=reports_array[0].keys())

    writer.writeheader()

    writer.writerows(reports_array)

    csv_content = csv_buffer.getvalue()

    csv_buffer.close()

    return csv_content

def generate_scheduled_events_csv(scheduled_events, election_event_id):
        events_array = [
            {
                "id": json.dumps(event["id"]),
                "tenant_id": json.dumps(event["tenant_id"]),
                "election_event_id": json.dumps(election_event_id),
                "created_at": json.dumps(event["created_at"]),
                "stopped_at": "null",
                "archived_at": "null",
                "labels": "null",
                "annotations": "null",
                "event_processor": json.dumps(event["event_processor"]),
                "cron_config": json.dumps(event["cron_config"]),
                "event_payload": json.dumps(event["event_payload"]),
                "task_id": json.dumps(event["task_id"])
            } for event in scheduled_events
        ]
        # Create an in-memory file-like object
        csv_buffer = io.StringIO()
        
        # Writing to the in-memory file
        writer = csv.DictWriter(csv_buffer, fieldnames=events_array[0].keys())

        # Write the header row
        writer.writeheader()

        # Write the rows
        writer.writerows(events_array)

        # Retrieve the CSV content as a string
        csv_content = csv_buffer.getvalue()

        # Close the StringIO object (optional for cleanup)
        csv_buffer.close()

        return csv_content

def create_tenant_conigurations_csv(tenant_teamplte_str):
    tenant_config = {
        "id": json.dumps(tenant_teamplte_str["id"]),
        "slug": json.dumps(tenant_teamplte_str["slug"]),
        "created_at": json.dumps(tenant_teamplte_str["created_at"]),
        "updated_at": json.dumps(tenant_teamplte_str["created_at"]),
        "labels": json.dumps(tenant_teamplte_str["labels"]),
        "annotations": json.dumps(tenant_teamplte_str["annotations"]),
        "is_active": json.dumps(tenant_teamplte_str["is_active"]),
        "voting_channels": json.dumps(tenant_teamplte_str["voting_channels"]),
        "settings": json.dumps(tenant_teamplte_str["settings"]),
        "test": json.dumps(tenant_teamplte_str["test"]),
    }

    csv_buffer = io.StringIO()

    writer = csv.DictWriter(csv_buffer, fieldnames=tenant_config.keys())

    writer.writeheader()

    writer.writerow(tenant_config)

    csv_content = csv_buffer.getvalue()

    csv_buffer.close()

    return csv_content
   


def create_tenant_files(excel_data, base_config):
    ## Load keycloak admin template
    keycloak_compiled = compiler.compile(keycloak_admin_template)
    keycloak = json.loads(keycloak_compiled({}))
    ## Load tenant configurations tamplte   
    tenant_configuration_compiled = compiler.compile(tenant_configurations)
    tenant_configuration_context = {
        "UUID": base_config["tenant_id"],
        "current_timestamp": current_timestamp
    }
    tenant_configurations_str = json.loads(tenant_configuration_compiled(tenant_configuration_context))

    final_json = {
        "tenant_configurations": tenant_configurations_str,
        "keycloak_admin_realm": keycloak
    }
    #Patch tenant config + keycloak admin realm with excel data parameters
    patch_json_with_excel(excel_data, final_json, "admin")

    keycloak = final_json["keycloak_admin_realm"]
    keycloak = patch_keycloak(keycloak, base_config)
    final_json["keycloak_admin_realm"] = keycloak
    
    permissions = excel_data["permissions"]
    try:
        # Create a zip file to store the CSV files
        zip_filename = f"output/tenants.zip"

        with zipfile.ZipFile(zip_filename, 'w', zipfile.ZIP_DEFLATED) as zipf:
                # Create permissions csv file
                filename = f"export_permissions.csv"
                csv_content =  create_permissions_file(permissions)
                zipf.writestr(filename, csv_content)

                filename = f"tenant_configurations.csv"
                csv_content = create_tenant_conigurations_csv(final_json["tenant_configurations"])
                zipf.writestr(filename, csv_content)

                json_str = json.dumps(final_json["keycloak_admin_realm"])
                csv_buffer = io.StringIO()
                csv_buffer.write(json_str)

                filename=f"keycloak_admin.json"
                zipf.writestr(filename, csv_buffer.getvalue())
                csv_buffer.close()
        
        print(f"ZIP file '{zip_filename}' created successfully with {len(permissions)} JSON files.")
    except Exception as e:
        logging.exception("An error occurred while creating the tenants ZIP file.")

def create_csv_files(final_json, scheduled_events, reports):
    try:
        # Create a zip file to store the CSV files
        election_event_id = final_json["election_event"]["id"]
        zip_filename = f"output/election-event.zip"

        with zipfile.ZipFile(zip_filename, 'w', zipfile.ZIP_DEFLATED) as zipf:            
            # Add the CSV files to the zip archive with a unique name
            filename = f"export_scheduled_events-{election_event_id}.csv"
            csv_content = generate_scheduled_events_csv(scheduled_events, election_event_id)
            zipf.writestr(filename, csv_content)

            filename = f"export_reports-{election_event_id}.csv"
            csv_content = generate_reports_csv(reports, election_event_id)
            zipf.writestr(filename, csv_content)

            ###### Add event
            # Convert to JSON string
            json_str = json.dumps(final_json)
            
            # Create an in-memory file-like object
            csv_buffer = io.StringIO()
            csv_buffer.write(json_str)
            
            # Add the CSV file to the zip archive with a unique name
            filename = f"export_election_event-{election_event_id}.json"
            zipf.writestr(filename, csv_buffer.getvalue())
            csv_buffer.close()
        
        print(f"ZIP file '{zip_filename}' created successfully with {len(scheduled_events)} CSV files.")
    except Exception as e:
        logging.exception("An error occurred while creating the scheduled events ZIP file.")

def process_excel_users(users, csv_data):
    users_map = {}
    for user in users:
        username = user["username"]
        if not username in users_map:
            users_map[username] = {
                "permission_labels": [],
                "username": None,
                "first_name": None,
                "enabled": None,
                "group_name": None,
                "password": None,
                "trustee": None
            }
        if "permission_labels" in user:
            users_map[username]["permission_labels"].append( user["permission_labels"] )
        if "username" in user:
            users_map[username]["username"] = user["username"]
        if "first_name" in user:
            users_map[username]["first_name"] = user["first_name"]
        if "enabled" in user and user["enabled"] is not None:
            users_map[username]["enabled"] =  user["enabled"]
        if "group_name" in user:
            users_map[username]["group_name"] = user["group_name"]
        if "password" in user:
            users_map[username]["password"] = user["password"]

    for user_data in users_map.values():
        if (
            user_data["enabled"] is None and
            (user_data["first_name"] is None or user_data["first_name"] == "") and
            (user_data["username"] is None or user_data["username"] == "") and
            len(user_data["permission_labels"]) == 0 and
            (user_data["group_name"] is None or user_data["group_name"] == "") and 
            (user_data["password"] is None or user_data["password"] == "") and
            (user_data["trustee"] is None or user_data["trustee"] == "")
        ):
            continue

        # deduplicate permission labels
        user_data["permission_labels"] = list(set(user_data["permission_labels"]))
        csv_data.append([
            user_data["enabled"],
            user_data["first_name"],
            user_data["username"],
            "|".join(user_data["permission_labels"]),
            user_data["password"],
            user_data["group_name"],
            user_data["trustee"]
        ])

def process_sbei_users(sbei_users, csv_data):
    users_map = {}
    for user in sbei_users:
        username = user["username"]
        users_map[username] = user
    
    for key_username in users_map.keys():
        # deduplicate permission labels
        permission_labels = list(set(users_map[key_username]["permission_label"]))
        trustee_id = users_map[key_username]["trustee_id"]
        csv_data.append([
            True,
            key_username,
            key_username,
            "|".join(permission_labels),
            key_username,
            "trustee" if key_username.startswith("trustee") else "sbei",
            trustee_id,
        ])

def create_permissions_file(data):
    roles_permissions = {}
    for row in data:
        for role, value in row.items():
            if role == "permissions":
                continue 

            if role not in roles_permissions:
                roles_permissions[role] = []

            if value == 'X':
                roles_permissions[role].append(row["permissions"])

    csv_data = [["role", "permissions"]]
    for role, permissions in roles_permissions.items():
        permissions_str = "|".join(permissions)
        csv_data.append([role, permissions_str])

    csv_buffer = io.StringIO()
    writer = csv.writer(csv_buffer)
    writer.writerows(csv_data)
    csv_content = csv_buffer.getvalue()
    csv_buffer.close()
    return csv_content


def create_admins_file(sbei_users, excel_data_users):
    # Data to be written to the CSV file
    print("excel_data_users", excel_data_users)
    csv_data = [
        [
            "enabled","first_name","username","permission_labels","password","group_name","trustee"
            #true,Eduardo,admin2,BANGKOK|DHAKA,admin2,admin
        ]
    ]
    process_excel_users(excel_data_users, csv_data)
    process_sbei_users(sbei_users, csv_data)


    # Name of the output CSV file
    csv_filename = "output/admins.csv"

    # Writing data to CSV file
    with open(csv_filename, mode='w', newline='') as file:
        writer = csv.writer(file)
        
        # Write each row from the data list
        writer.writerows(csv_data)

    print(f"CSV file '{csv_filename}' created successfully.")

def create_voters_file(sqlite_output_path):
    voters_sql = get_voters(sqlite_output_path)
    # Data to be written to the CSV file
    csv_data = [
        [
             #"enabled", "first_name", "last_name", "birthDate", "area_name", "embassy", "country", "group_name"
             "enabled", "first_name", "last_name", "middleName",  "dateOfBirth", "area_name", "embassy", "country", "group_name"
        ]
    ]
    for row in voters_sql:
        #embassy = get_embassy(row["DB_POLLING_CENTER_POLLING_PLACE"])
        #country_code = row["voter_COUNTRY"]
        embassy = "Dhaka PE" if "BD" ==  row["voter_COUNTRY"] else "Bangkok PE"
        country = "Bangladesh/Dhaka PE" if "BD" ==  row["voter_COUNTRY"] else "Thailand/Bangkok PE"
        area_name = "PEOPLES REPUBLIC OF BANGLADESH" if "BD" ==  row["voter_COUNTRY"] else "KINGDOM OF THAILAND"
        csv_data.append([
            "TRUE",
            row["voter_FIRSTNAME"].title(),
            row["voter_LASTNAME"].title(),
            row["voter_MATERNALNAME"].title(),
            row["voter_DATEOFBIRTH"],
            #get_country_from_area_embassy(row["DB_ALLMUN_AREA_NAME"], embassy),
            area_name,
            embassy,
            country,
            "voter"
        ])

    # Name of the output CSV file
    csv_filename = "output/voters.csv"

    # Writing data to CSV file
    with open(csv_filename, mode='w', newline='') as file:
        writer = csv.writer(file)
        
        # Write each row from the data list
        writer.writerows(csv_data)

    print(f"CSV file '{csv_filename}' created successfully.")
        

def gen_keycloak_context(excel_data, areas_dict):
    print(f"generating keycloak context")
    country_set = set()
    embassy_set = set()

    for _region_code, areas in areas_dict.items():
        for area in areas:
            country = get_embassy(area["name"]).split("-")[0].strip()
            embassy = get_embassy(area["description"])
            embassy_set.add("\\\"" + embassy + "\\\"")
            country_set.add("\\\"" + country + "/" + embassy + "\\\"")

    sorted_embassy_list = sorted(embassy_set)
    sorted_country_list = sorted(country_set)
    
    keycloak_settings = [t for t in excel_data["parameters"] 
                         if t["type"] == "settings" and t["key"].startswith("keycloak")]
    keycloak_context = {
        "embassy_list": ",".join(sorted_embassy_list),
        "country_list": ",".join(sorted_country_list),
    }

    key_mappings = {
        "philis_id_inetum_min_value_documental_score": "keycloak_inetum_min_value_philis_id_documental_score",
        "philis_id_inetum_min_value_facial_score": "keycloak_inetum_min_value_philis_id_facial_score",
        "seaman_book_inetum_min_value_val_campos_criticos_score": "keycloak_inetum_min_value_seaman_book_val_campos_criticos_score",
        "seaman_book_inetum_min_value_facial_score": "keycloak_inetum_min_value_seaman_book_facial_score",
        "passport_inetum_min_value_val_campos_criticos_score": "keycloak_inetum_min_value_passport_val_campos_criticos_score",
        "passport_inetum_min_value_facial_score": "keycloak_inetum_min_value_passport_facial_score",
        "driver_license_inetum_min_value_val_campos_criticos_score": "keycloak_inetum_min_value_driver_license_val_campos_criticos_score",
        "driver_license_inetum_min_value_facial_score": "keycloak_inetum_min_value_driver_license_facial_score",
        "ibp_inetum_min_value_val_campos_criticos_score": "keycloak_inetum_min_value_ibp_val_campos_criticos_score",
        "ibp_inetum_min_value_facial_score": "keycloak_inetum_min_value_ibp_facial_score",
    }

    keycloak_settings_dict = {row["key"]: row["value"] for row in keycloak_settings}

    for context_key, settings_key in key_mappings.items():
        keycloak_context[context_key] = int(keycloak_settings_dict.get(settings_key, 50))
    return keycloak_context

def load_sqlite_query(script_dir):
    ocf_path = get_data_ocf_path(script_dir)
    precinct_ids = list_folders(ocf_path)

    precinct_id = precinct_ids[0]
    sqlite_output_path = os.path.join(ocf_path, precinct_id, "db_sqlite_miru.db")
    results = get_data(sqlite_output_path, excel_data)
    return results

def gen_tree(excel_data, miru_data, results, multiply_factor):
    elections_object = {"elections": []}

    ccs_servers = {}

    # areas, indexed by region code
    areas = {}
    for row in results:
        precinct_id = row["DB_TRANS_SOURCE_ID"]

        area_context = next((area for area in excel_data["areas"] if str(area["precinct_id"]) == precinct_id), None)

        if area_context is None:
            raise Exception(f"precinct with 'id' = {precinct_id} not found in excel areas/precincts tab")
        area_name = area_context["name"]
        area_split = area_name.split("-")
        if len(area_split) < 2:
            raise Exception(f"Invalid area name = {area_name} expected a dash")
        area_description = area_split[1].strip()

        if precinct_id not in miru_data:
            raise Exception(f"precinct with 'id' = {precinct_id} not found in miru acf")
        miru_precinct = miru_data[precinct_id]
        registered_voters = miru_precinct["REGISTERED_VOTERS"]

        region_code = str(area_context["region_code_overwrite"] if "region_code_overwrite" in area_context and area_context["region_code_overwrite"] is not None else row["pop_POLLCENTER_CODE"])

        # the area
        if region_code in areas:
            found_area = next((a for a in areas[region_code] if a["precinct_id"] == precinct_id), None)
            if found_area:
                continue

        ccs_servers = [{
            "name": server["NAME"],
            "tag": server["ID"],
            "address": server["ADDRESS"],
            "public_key_pem": server["PUBLIC_KEY"],
            "send_logs": "CENTRAL" == server["TYPE"],
        } for server in miru_precinct["SERVERS"].values()]

        sbei_ids = [user["ID"] for user in miru_precinct["USERS"]]
        sbei_ids_str = json.dumps(sbei_ids)
        sbei_ids_str = sbei_ids_str.replace('"', '\\"')

        ccs_servers_str = json.dumps(ccs_servers)
        ccs_servers_str = ccs_servers_str.replace('"', '\\"').replace('\\n', '\\\\n')

        area = {
            "name": area_name,
            "description" : area_description,
            "source_id": row["DB_TRANS_SOURCE_ID"],
            "region_code": str(region_code),
            "precinct_id": str(precinct_id),
            **base_context,
            "miru": {
                "ccs_servers": ccs_servers_str,
                "sbei_ids": sbei_ids_str,
                "country": row["DB_ALLMUN_AREA_NAME"],
                "registered_voters": registered_voters,
                "station_name": row["DB_PRECINCT_ESTABLISHED_CODE"]
            }
        }
        if region_code not in areas:
            areas[region_code] = []
        areas[region_code].append(area)

    for (idx, row) in enumerate(results):
        # print(f"processing row {idx}")
        # Find or create the election object

        precinct_id = row_precinct_id = row["DB_TRANS_SOURCE_ID"]

        # each post has a precinct id, find the region code for that precinct in the areas/precincts tab
        area_context = next((area for area in excel_data["areas"] if str(area["precinct_id"]) == precinct_id), None)
        if area_context is None:
            raise Exception(f"precinct with 'id' = {precinct_id} not found in excel areas/precincts tab")
        region_code = str(area_context["region_code_overwrite"] if "region_code_overwrite" in area_context and area_context["region_code_overwrite"] is not None else row["pop_POLLCENTER_CODE"])

        row_election_post = row["DB_POLLING_CENTER_POLLING_PLACE"]
        election = next((e for e in elections_object["elections"] if e["precinct_id"] == row_precinct_id), None)
        election_context = next((
            c for c in excel_data["elections"] 
            if str(c["precinct_id"]) == row_precinct_id
        ), None)

        if election_context is None:
            # it's a precinct, not a post, filter out
            continue

        election_context["precinct_id"] = str(election_context["precinct_id"])

        if not election_context:
            raise Exception(f"election with 'precinct_id' = {row_precinct_id} not found in excel")
        
        if precinct_id not in miru_data:
            raise Exception(f"precinct with 'id' = {precinct_id} not found in miru acf")
        miru_precinct = miru_data[precinct_id]

        if not election:
            contest_id = row["DB_SEAT_DISTRICTCODE"]
            if not contest_id in miru_precinct["CONTESTS"]:
                raise Exception(f"contest with 'id' = {contest_id} and precinct = {precinct_id} not found in miru acf")
            miru_contest = miru_precinct["CONTESTS"][contest_id]
            # If the election does not exist, create it
            election = {
                "election_post": row_election_post,
                "precinct_id": precinct_id,
                "region_code": region_code,
                "election_name": election_context["name"],
                "contests": [],
                "scheduled_events": [],
                "reports": [],
                "miru": {
                    "election_id": "1",
                    "name": miru_contest["NAME_ABBR"],
                    "post": row_election_post,
                    "geographical_region": miru_precinct["REGION"],
                    "precinct_code": row["DB_PRECINCT_ESTABLISHED_CODE"],
                    "pollcenter_code": row["DB_TRANS_SOURCE_ID"]
                },
                **base_context,
                **election_context
            }
            elections_object["elections"].append(election)

        # Find or create the contest object within the election
        contest_name = row["DB_CONTEST_NAME"]
        contest = next((c for c in election["contests"] if c["name"] == contest_name), None)
        
        if not contest:
            # If the contest does not exist, create it
            contest = {
                "name": contest_name,
                **base_context,
                "eligible_amount": row["DB_RACE_ELIGIBLEAMOUNT"],
                "district_code": row["DB_SEAT_DISTRICTCODE"],
                "sort_order": row["contest_SORT_ORDER"],
                "candidates": []
            }
            election["contests"].append(contest)

        # Add the candidate to the contest
        candidate_name = row["DB_CANDIDATE_NAMEONBALLOT"]
        candidate_id = row["DB_CANDIDATE_CAN_CODE"]
        if not candidate_id in miru_precinct["CANDIDATES"]:
            raise Exception(f"candidate with 'id' = {candidate_id} and precinct = {precinct_id} not found in miru acf")
        miru_candidate = miru_precinct["CANDIDATES"][candidate_id]

        starts1 = str(miru_candidate["DISPLAY_ORDER"]) + " "
        starts2 = str(miru_candidate["DISPLAY_ORDER"]) + ". "
        if not (candidate_name.startswith(starts1) or candidate_name.startswith(starts2)):
            candidate_name = str(miru_candidate["DISPLAY_ORDER"]) + ". " + candidate_name

        candidate = {
            "code": candidate_id,
            "name_on_ballot": candidate_name,
            "party_short_name": miru_candidate["PARTY_NAME_ABBR"],
            "party_name": miru_candidate["PARTY_NAME"],
            "DB_CANDIDATE_NAMEONBALLOT": row["DB_CANDIDATE_NAMEONBALLOT"],
            **base_context,
            "miru": {
                "candidate_affiliation_id": miru_candidate["PARTY_ID"],
                "candidate_affiliation_party": miru_candidate["PARTY_NAME"],
                "candidate_affiliation_registered_name": miru_candidate["PARTY_NAME_ABBR"],
            },
            "sort_order": miru_candidate["DISPLAY_ORDER"],
        }
        found_candidate = next((
            c for c in contest["candidates"]
            if c["code"] == candidate["code"] and
            c["name_on_ballot"] == candidate["name_on_ballot"] and
            c["party_name"] == candidate["party_name"]),
        None)

        if found_candidate is None:
            contest["candidates"].append(candidate)

    # test elections
    test_elections =  copy.deepcopy(elections_object["elections"])
    for election in test_elections:
        election["name"] = "Test Voting"
        name_parts = election["alias"].split("-")
        parts_join = [name_parts[0].strip()]
        if len(name_parts) > 1 and "GENERAL" not in name_parts[1].upper():
            parts_join.append(name_parts[1].strip())
        parts_join.append("Test Voting")
        election["alias"] = " - ".join(parts_join)

    elections_object["elections"].extend(test_elections)
    
    def is_element_match_election(element, election):
        is_regex = is_valid_regex(element["election_alias"])

        if is_regex:
            return re.match(element["election_alias"],  election["alias"])
        
        if not isinstance(element["election_alias"], str) or not isinstance(election["alias"], str):
            return False
        
        return element["election_alias"].strip() == election["alias"].strip()

    # scheduled events
    for election in elections_object["elections"]:
        election_scheduled_events = [
            scheduled_event
            for scheduled_event
            in excel_data["scheduled_events"] 
            if is_element_match_election(scheduled_event, election)
        ]
        election["scheduled_events"] = election_scheduled_events

    for election in elections_object["elections"]:
        election_reports = []
        for report in excel_data["reports"]:
            if not is_element_match_election(report, election):
                continue
            permission_labels = []
            if report["permission_label"]:
                permission_labels = report["permission_label"].split("|")
            if election["permission_label"]:
                permission_labels.append(election["permission_label"])
            report_clone = report.copy()
            report_clone["permission_label"] = "|".join(permission_labels)

            election_reports.append(report_clone)
            
        election["reports"] = election_reports
    
    original_elections = copy.deepcopy(elections_object["elections"])
    for i in range(1, multiply_factor):
        duplicated_elections = copy.deepcopy(original_elections)
        for election in duplicated_elections:
            election["name"] += f" {i}"
            print(f"election name {i}", election["name"])
            election["alias"] += f" {i}"
            for contest in election["contests"]:
                contest["name"] += f" {i}"
                for candidate in contest["candidates"]:
                    candidate["name_on_ballot"] += f" {i}"
        elections_object["elections"].extend(duplicated_elections)

    print(f"elections_object {len(elections_object['elections'])}")

    return elections_object, areas

def replace_placeholder_database(excel_data, election_event_id, miru_data, results, multiply_factor):
    election_tree, areas_dict = gen_tree(excel_data, miru_data, results, multiply_factor)
    keycloak_context = gen_keycloak_context(excel_data, areas_dict)

    election_compiled = compiler.compile(election_template)
    contest_compiled = compiler.compile(contest_template)
    candidate_compiled = compiler.compile(candidate_template)
    area_compiled = compiler.compile(area_template)
    area_contest_compiled = compiler.compile(area_contest_template)
    scheduled_event_compiled = compiler.compile(scheduled_event_template)
    reports_compiled = compiler.compile(reports_template)

    area_contests = []
    area_contexts_dict = {}
    areas = []
    candidates = []
    contests = []
    elections = []
    scheduled_events = []
    reports = []

    print(f"rendering keycloak")
    keycloak_render = render_template(keycloak_template, keycloak_context)
    keycloak = json.loads(keycloak_render)
    

    for election in election_tree["elections"]:
        election_id = generate_uuid()
        election_context = {
            **election,
            "UUID": election_id,
            "tenant_id": base_config["tenant_id"],
            "election_event_id": election_event_id,
            "current_timestamp": current_timestamp,
            "DB_POLLING_CENTER_POLLING_PLACE": election["election_post"],
            "election_post": election["election_post"],
            "election_name": election["election_name"]
        }

        print(f"rendering election {election['election_name']}")
        elections.append(json.loads(election_compiled(election_context)))

        for scheduled_event in election["scheduled_events"]:
            scheduled_event_id = generate_uuid()
            scheduled_event_context = {
                "UUID": scheduled_event_id,
                "tenant_id": base_config["tenant_id"],
                "election_event_id": election_event_id,
                "election_id": election_context["UUID"],
                "election_alias": scheduled_event["election_alias"],
                "event_processor": scheduled_event["type"],
                "scheduled_date": scheduled_event["date"],
                "current_timestamp": current_timestamp
            }
            print(f"rendering scheduled event {scheduled_event_context['election_alias']} {scheduled_event_context['event_processor']}")
            scheduled_events.append(json.loads(scheduled_event_compiled(scheduled_event_context)))

        for report in election["reports"]:
            report_id = generate_uuid()
            report_context = {
                **report,
                "UUID": report_id,
                "tenant_id": base_config["tenant_id"],
                "election_event_id": election_event_id,
                "current_timestamp": current_timestamp,
                "is_cron_active": report["is_cron_active"] if report["is_cron_active"] else False,
                "cron_expression": report["cron_expression"],
                "template_alias": report["template_alias"],
                "encryption_policy": report["encryption_policy"],
                "email_recipients": json.dumps((report["email_recipients"].split(",") if report["email_recipients"] else [])),
                "report_type": report["report_type"],
                "election_id": election_context["UUID"],
                "password": report["password"],
                "permission_label": report["permission_label"],
            }

            print(f"rendering report {report_context['UUID']}")
            reports.append(json.loads(reports_compiled(report_context)))

        # find the areas:
        region_code = election["region_code"]
        precinct_id = election["precinct_id"]
        if region_code not in areas_dict:
            raise Exception(f"election  with 'id' = {precinct_id} has no found areas/precincts")
        election_areas = areas_dict[region_code]

        for contest in election["contests"]:
            contest_id = generate_uuid()
            contest_context = {
                **contest,
                "max_votes": contest["eligible_amount"],
                "UUID": contest_id,
                "tenant_id": base_config["tenant_id"],
                "election_event_id": election_event_id,
                "election_id": election_context["UUID"],
                "DB_CONTEST_NAME": contest["name"],
                "DB_RACE_ELIGIBLEAMOUNT": contest["eligible_amount"],
                "DB_POLLING_CENTER_POLLING_PLACE": election["election_post"],
                "current_timestamp": current_timestamp
            }

            print(f"rendering contest {contest['name']}")
            contests.append(json.loads(contest_compiled(contest_context)))

            for candidate in contest["candidates"]:
                candidate_context = {
                    **candidate,
                    "UUID": generate_uuid(),
                    "tenant_id": base_config["tenant_id"],
                    "election_event_id": election_event_id,
                    "contest_id": contest_context["UUID"],
                    "sort_oder": candidate["sort_order"],
                }

                print(f"rendering candidate {candidate['name_on_ballot']}")
                candidates.append(json.loads(candidate_compiled(candidate_context)))

            for area in election_areas:
                area_precinct_id = area["precinct_id"]

                if area_precinct_id not in area_contexts_dict:
                    area_context = {
                        **area,
                        "UUID": generate_uuid(),
                        "tenant_id": base_config["tenant_id"],
                        "election_event_id": election_event_id,
                        "DB_TRANS_SOURCE_ID": area["source_id"],
                        "DB_ALLMUN_AREA_NAME": area["name"],
                        "DB_POLLING_CENTER_POLLING_PLACE":area["description"]
                    }
                    area_contexts_dict[area_precinct_id] = area_context

                    print(f"rendering area {area['name']}")
                    areas.append(json.loads(area_compiled(area_context)))
                else:
                    area_context = area_contexts_dict[area_precinct_id]

                area_contest_context = {
                    "UUID": generate_uuid(),
                    "area_id": area_context["UUID"],
                    "contest_id": contest_context["UUID"]
                }

                print(f"rendering area_contest area: '{area['name']}', contest: '{contest['name']}'")
                area_contests.append(json.loads(area_contest_compiled(area_contest_context)))

    for report in excel_data["reports"]:
        if report["election_alias"]:
            continue
        report_id = generate_uuid()
        report_context = {
            **report,
            "UUID": report_id,
            "tenant_id": base_config["tenant_id"],
            "election_event_id": election_event_id,
            "current_timestamp": current_timestamp,
            "is_cron_active": report["is_cron_active"] if report["is_cron_active"] else False,
            "cron_expression": report["cron_expression"],
            "template_alias": report["template_alias"],
            "encryption_policy": report["encryption_policy"],
            "email_recipients": json.dumps((report["email_recipients"].split(",") if report["email_recipients"] else [])),
            "report_type": report["report_type"],
            "password": report["password"],
            "permission_label": report["permission_label"],
            }

        print(f"rendering report {report_context['UUID']}")
        reports.append(json.loads(reports_compiled(report_context)))
    
    for scheduled_event in excel_data["scheduled_events"]:
        if scheduled_event["election_alias"]:
            continue
        scheduled_event_id = generate_uuid()
        scheduled_event_context = {
            "UUID": scheduled_event_id,
            "tenant_id": base_config["tenant_id"],
            "election_event_id": election_event_id,
            "election_id": None,
            "election_alias": scheduled_event["election_alias"],
            "event_processor": scheduled_event["type"],
            "scheduled_date": scheduled_event["date"],
            "current_timestamp": current_timestamp
        }
        print(f"rendering scheduled event {scheduled_event_context['event_processor']}")
        scheduled_events.append(json.loads(scheduled_event_compiled(scheduled_event_context)))

    return areas, candidates, contests, area_contests, elections, keycloak, scheduled_events, reports


def parse_election_event(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^logo_url$"
        ],
        allowed_keys=[
            "^logo_url$",
            "^root_ca$",
            "^intermediate_cas$",
        ]
    )
    event = data[0]
    event["root_ca"] = event["root_ca"].replace('\n', '\\n')
    event["intermediate_cas"] = event["intermediate_cas"].replace('\n', '\\n')
    return data[0]

def parse_users(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^username$"
        ],
        allowed_keys=[
            "^username$",
            "^first_name$",
            "^enabled$",
            "^group_name",
            "^permission_labels$",
            "^password$",
        ]
    )
    return data

def parse_posts(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^precinct_id$",
            "^description$"
        ],
        allowed_keys=[
            "^precinct_id$",
            "^description$",
            "^permission_label$",
            "^trustees$",
        ]
    )
    return data

def parse_precincts(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^precinct_id$",
            "^name$"
        ],
        allowed_keys=[
            "^precinct_id$",
            "^name$",
            "^region_code_overwrite$"
        ]
    )
    return data

def parse_reports(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^report_type$",
        ],
        allowed_keys=[
            "^election_alias$",
            "^template_alias$",
            "^encryption_policy$",
            "^is_cron_active$",
            "^email_recipients",
            "^cron_expression$",
            "^report_type$",
            "^password$",
            "^permission_label$",
        ]
    )
    return data

def parse_scheduled_events(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^election_alias$",
            "^type$",
            "^date$"
        ],
        allowed_keys=[
            "^election_alias$",
            "^type$",
            "^date$"
        ]
    )
    return data

def parse_permissions(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^permissions$",
            "^admin$",
            "^sbei$",
            "^trustee$",
        ],
        allowed_keys=[
            "^permissions$",
            "^admin$",
            "^sbei$",
            "^trustee$",
            "^.*$",
        ]
    )
    print(f"parse_permissions {data}")
    return data

def parse_excel(excel_path):
    '''
    Parse all input files specified in the config file into their respective
    data structures.
    '''
    electoral_data = openpyxl.load_workbook(excel_path)

    return dict(
        election_event = parse_election_event(electoral_data['ElectionEvent']),
        elections = parse_posts(electoral_data['Posts']),
        areas = parse_precincts(electoral_data['Precincts']),
        scheduled_events = parse_scheduled_events(electoral_data['ScheduledEvents']),
        reports = parse_reports(electoral_data['Reports']),
        users = parse_users(electoral_data['Users']),
        parameters = parse_parameters(electoral_data['Parameters']),
        permissions = parse_permissions(electoral_data['Permissions'])
    )


def list_folders(directory):
    return [name for name in os.listdir(directory) if os.path.isdir(os.path.join(directory, name))]

def index_by(array, id):
    return {obj[id]: obj for obj in array}

def read_json_file(file_path):
    # Load and prepare each section template
    try:
        with open(file_path, 'r') as file:
            data = json.loads(file.read())
            return data

        logging.info(f"Loaded {file_path} successfully.")
    except FileNotFoundError as e:
        logging.exception(f"File not found: {e}")
    except Exception as e:
        logging.exception("An error occurred while loading templates.")
    return

def read_text_file(file_path):
    # Load and prepare each section template
    try:
        with open(file_path, 'r') as file:
            return file.read()

        logging.info(f"Loaded {file_path} successfully.")
    except FileNotFoundError as e:
        logging.exception(f"File not found: {e}")
    except Exception as e:
        logging.exception("An error occurred while loading templates.")
    return

def calculate_sha256(input_string):
    # Compute the SHA-256 hash and return it in uppercase
    return hashlib.sha256(input_string.encode('utf-8')).hexdigest().upper()


def extract_zip(zip_file, password, output_folder):
    # Ensure the output folder exists
    os.makedirs(output_folder, exist_ok=True)

    with pyzipper.AESZipFile(zip_file) as zf:
        if password:
            zf.setpassword(password.encode('utf-8'))
        # Extract all files into the specified output folder
        zf.extractall(path=output_folder)
        print(f"Files extracted successfully to: {output_folder}")

def get_data_ocf_path(script_dir):
    return os.path.join(script_dir, "data", "ocf")

def extract_miru_zips(acf_path, script_dir):
    ocf_path = get_data_ocf_path(script_dir)
    if IS_DEBUG:
        return ocf_path
    remove_folder_if_exists(ocf_path)
    assert_folder_exists(ocf_path)
    extract_zip(acf_path, None, ocf_path)

    ocf_zips =  [name for name in os.listdir(ocf_path) if os.path.isfile(os.path.join(ocf_path, name))]

    for zip_file_name in ocf_zips:
        folder_name = Path(zip_file_name).stem
        input_string = f"ocf#({folder_name})#$"
        zip_password = calculate_sha256(input_string)
        ocf_folder_path = os.path.join(ocf_path, folder_name)
        zip_file_path = os.path.join(ocf_path, zip_file_name)
        assert_folder_exists(ocf_folder_path)
        extract_zip(zip_file_path, zip_password, ocf_folder_path)
        remove_file_if_exists(zip_file_path)

    return ocf_path

def patch_keycloak(keycloak, base_config):
    css = keycloak["localizationTexts"]["en"]["loginCustomCss"]
    for key, value in base_config["replacements"].items():
        css = css.replace(key, value)
    
    keycloak["localizationTexts"]["en"]["loginCustomCss"] = css
    return keycloak

def read_miru_data(acf_path, script_dir):
    ocf_path = extract_miru_zips(acf_path, script_dir)
    data = {} # read_inspector_pwds
    folders = list_folders(ocf_path)
    for precinct_id in folders:
        precinct_file = read_json_file(os.path.join(ocf_path, precinct_id, 'precinct.ocf'))
        security_file = read_json_file(os.path.join(ocf_path, precinct_id, 'security.ocf'))
        server_file = read_json_file(os.path.join(ocf_path, precinct_id, 'server.ocf'))

        servers = index_by(server_file["SERVERS"], "ID")
        security = index_by(security_file["CERTIFICATES"], "ID")
        keystore_path = os.path.join(ocf_path, precinct_id, 'keystore.bks')

        users = []
        
        if not args.only_voters:
            print(f"Reading keys for precint {precinct_id}")

            for certificate in security.values():
                if "USER" == certificate["TYPE"]:
                    full_id = certificate["ID"] # example: eb_91070001-01
                    user_data = certificate["ID"].split("-")
                    user_role = user_data[1]
                    if "07" == user_role:
                        continue
                    
                    users.append({
                        "ID": full_id,
                        "NAME": certificate["NAME"],
                        "ROLE": user_role,
                        "INPUT_NAME": True
                    })
        
        for server in servers.values():
            server_id = server["ID"]
            alias = security[server_id]["ALIAS"]
            alias_path = f"data/{alias}.pem"
            if args.only_voters:
                server["PUBLIC_KEY"] = ""
                continue
            command = f"""keytool -exportcert \
                -alias {alias} \
                -keystore {keystore_path} \
                -storetype BKS \
                -storepass "" \
                -providerclass org.bouncycastle.jce.provider.BouncyCastleProvider \
                -providerpath bcprov.jar \
                -rfc \
                | openssl x509 -pubkey -noout > {alias_path}"""
            if not IS_DEBUG:
                run_command(command, script_dir)
            
            alias_pubkey = read_text_file(alias_path)
            server["PUBLIC_KEY"] = alias_pubkey

        election = precinct_file["ELECTIONS"][0]
        region = next((e for e in precinct_file["REGIONS"] if e["TYPE"] == "Province"), None)
        registered_voters = precinct_file["POLLING_STATION"]["VOTER_COUNT"]

        precinct_data = {
            "EVENT_ID": election["EVENT_ID"],
            "EVENT_NAME": election["NAME"],
            "CONTESTS": index_by(precinct_file["CONTESTS"], "ID"),
            "CANDIDATES": index_by(precinct_file["CANDIDATES"], "ID"),
            "REGIONS": precinct_file["REGIONS"],
            "REGION": region["NAME"],
            "REGISTERED_VOTERS": registered_voters,
            "SERVERS": servers,
            "USERS": users
        }
        data[precinct_id] = precinct_data

        if IS_DEBUG:
            continue

        sql_output_path = os.path.join(ocf_path, precinct_id,'miru.sql')
        sqlite_output_path =  os.path.join(ocf_path, precinct_id, 'db_sqlite_miru.db')
        sql_input_path = os.path.join(ocf_path, precinct_id,'ELECTION_DATA')
        remove_file_if_exists(sql_output_path)
        remove_file_if_exists(sqlite_output_path)
        render_sql(sql_input_path, sql_output_path, voters_path)
        # Convert MySQL dump to SQLite
        command = f"chmod +x mysql2sqlite && ./mysql2sqlite {sql_output_path} | sqlite3 {sqlite_output_path}"
        # Log the constructed command
        print(f"Command to convert MySQL dump to SQLite: {command}")

        run_command(command, script_dir)

    return data

# Step 0: ensure certain folders exist
remove_folder_if_exists("output")
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


# Step 2: Set up argument parsing
parser = argparse.ArgumentParser(description="Process a Miru zip file and an excel file, and generate an election event")
parser.add_argument('miru', type=str, help='Base name of zip file with the OCF files from Miru ')
parser.add_argument('excel', type=str, help='Excel config (with .xlsx extension)')
parser.add_argument('--voters', type=str, metavar='VOTERS_FILE_PATH', help='Create a voters file if this flag is set')
parser.add_argument('--only-voters', type=str, metavar='VOTERS_FILE_PATH', help='Only create a voters file if this flag is set')
parser.add_argument('--multiply-elections', type=int, default=1, help='Multiply the number of elections created by this factor')


# Step 3: Parse the arguments
args = parser.parse_args()

# Step 4: Use the filename argument in your command
miru_path = args.miru
logging.debug(f"Miru received: {miru_path}")

excel_path = args.excel
logging.debug(f"Excel received: {excel_path}")

voters_path = args.voters or args.only_voters or None

# Determine the script's directory to use as cwd
script_dir = os.path.dirname(os.path.abspath(__file__))

# Step 7: Read Excel
excel_data = parse_excel(excel_path)

miru_data = read_miru_data(miru_path, script_dir)

# Step 9: Read base configuration
base_config = read_base_config()

# Step 10: Get the current timestamp
current_timestamp = datetime.now(timezone.utc).isoformat()
logging.debug(f"Current timestamp: {current_timestamp}")

base_context = {
    "tenant_id": base_config["tenant_id"],
    "current_timestamp": current_timestamp
}

# Step 12: Compile and render templates using pybars3

# Load and prepare each section template
try:
    with open('templates/electionEvent.hbs', 'r') as file:
        election_event_template = file.read()

    with open('templates/election.hbs', 'r') as file:
        election_template = file.read()

    with open('templates/contest.hbs', 'r') as file:
        contest_template = file.read()

    with open('templates/candidate.hbs', 'r') as file:
        candidate_template = file.read()

    with open('templates/area.hbs', 'r') as file:
        area_template = file.read()

    with open('templates/areaContest.hbs', 'r') as file:
        area_contest_template = file.read()

    with open('templates/COMELEC/keycloak.hbs', 'r') as file:
        keycloak_template = file.read()

    with open('templates/scheduledEvent.hbs', 'r') as file:
        scheduled_event_template = file.read()
    
    with open('templates/report.hbs', 'r') as file:
        reports_template = file.read()

    with open('templates/tenantConfigurations.hbs') as file:
        tenant_configurations = file.read()
    
    with open('templates/COMELEC/keycloakAdmin.hbs', 'r') as file:
        keycloak_admin_template = file.read()
    

    logging.info("Loaded all templates successfully.")
except FileNotFoundError as e:
    logging.exception(f"Template file not found: {e}")
except Exception as e:
    logging.exception("An error occurred while loading templates.")

if args.only_voters:
    print("Only voters, exiting the script.")
    sys.exit()

multiply_factor = args.multiply_elections
results = load_sqlite_query(script_dir)
election_event, election_event_id, sbei_users = generate_election_event(excel_data, base_context, miru_data, results)
create_tenant_files(excel_data, base_config)
create_admins_file(sbei_users, excel_data["users"])

areas, candidates, contests, area_contests, elections, keycloak, scheduled_events, reports = replace_placeholder_database(excel_data, election_event_id, miru_data, results, multiply_factor)
keycloak = patch_keycloak(keycloak, base_config)

final_json = {
    "tenant_id": base_config["tenant_id"],
    "keycloak_event_realm": keycloak,  # Add appropriate value or leave it as is
    "election_event": election_event,  # Include the generated election event
    "elections": elections,  # Include the election objects
    "contests": contests,  # Include the contest objects
    "candidates": candidates, # Include the candidate objects
    "areas": areas,  # Include the area objects
    "area_contests": area_contests,  # Include the area-contest relationships
    "scheduled_events": scheduled_events,
    "reports": reports
}

patch_json_with_excel(excel_data, final_json, "event")

scheduled_events = final_json["scheduled_events"]
reports = final_json["reports"]

final_json["scheduled_events"] = None
final_json["reports"] = []

# Step 14: Save final ZIP to a file
try:
    # Create the scheduled events zip file after generating the final JSON
    create_csv_files(final_json, scheduled_events, reports)
    logging.info("Final ZIP generated and saved successfully.")
except Exception as e:
    logging.exception("An error occurred while saving the final JSON.")
