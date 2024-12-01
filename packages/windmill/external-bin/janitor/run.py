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
import zipfile
import io

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
        
    boc_members = parse_table_values(base_tables_path + 'BOCMembers.txt', 'boc_members', table_format['boc_members'] )
    candidates = parse_table_values(base_tables_path + 'Candidates.txt', 'candidates', table_format['candidates'] )
    ccs = parse_table_values(base_tables_path + 'CCS.txt', 'ccs', table_format['ccs'] )
    contest = parse_table_values(base_tables_path + 'Contest.txt', 'contest', table_format['contest'] )
    contest_class = parse_table_values(base_tables_path + 'Contest_Class.txt', 'contest_class', table_format['contest_class'] )
    eb_members = parse_table_values(base_tables_path + 'EBMembers.txt', 'eb_members', table_format['eb_members'] )
    political_organizations = parse_table_values(base_tables_path + 'Political_Organizations.txt', 'political_organizations', table_format['political_organizations'] )
    polling_centers = parse_table_values(base_tables_path + 'Polling_Centers.txt', 'polling_centers', table_format['polling_centers'] )
    polling_district_region = parse_table_values(base_tables_path + 'Polling_District_Region.txt', 'polling_district_region', table_format['polling_district_region'] )
    polling_district = parse_table_values(base_tables_path + 'Polling_District.txt', 'polling_district', table_format['polling_district'] )
    precinct_established = parse_table_values(base_tables_path + 'Precinct_Established.txt', 'precinct_established', table_format['precinct_established'] )
    precinct = parse_table_values(base_tables_path + 'Precinct.txt', 'precinct', table_format['precinct'] )
    region = parse_table_values(base_tables_path + 'Region.txt', 'region', table_format['region'] )
    voting_device = parse_table_values(base_tables_path + 'Voting_Device.txt', 'voting_device', table_format['voting_device'] )

    miru_context = {
        "boc_members": boc_members,
        "candidates": candidates,
        "ccs": ccs,
        "contest": contest,
        "contest_class": contest_class,
        "eb_members": eb_members,
        "political_organizations": political_organizations,
        "polling_centers": polling_centers,
        "polling_district_region": polling_district_region,
        "polling_district": polling_district,
        "precinct_established": precinct_established,
        "precinct": precinct,
        "region": region,
        "voting_device": voting_device,
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
        else:
            logging.error("Command failed.")
            logging.error(f"Error: {result.stderr}")
    except Exception as e:
        logging.exception("An error occurred while running the command.")


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
        return []
    
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
    precinct_ids = [e["precinct_id"] for e in excel_data["elections"]]
    precinct_ids_str = ",".join([f"'{precinct_id}'" for precinct_id in precinct_ids])

    query = f"""SELECT 
        region.REGION_CODE as pop_POLLCENTER_CODE,
        polling_centers.VOTING_CENTER_CODE as allbgy_ID_BARANGAY,
        polling_centers.VOTING_CENTER_NAME as allbgy_AREANAME,
        polling_centers.VOTING_CENTER_ADDR  as DB_ALLMUN_AREA_NAME,
        region.REGION_NAME as DB_POLLING_CENTER_POLLING_PLACE,
        voting_device.VOTING_DEVICE_CODE as DB_TRANS_SOURCE_ID,
        voting_device.UPPER_CCS as trans_route_TRANS_DEST_ID,
        polling_district.DESCRIPTION as DB_CONTEST_NAME,
        polling_district.POLLING_DISTRICT_NUMBER as DB_RACE_ELIGIBLEAMOUNT,
        polling_district.POLLING_DISTRICT_CODE as DB_SEAT_DISTRICTCODE,
        contest_class.PRECEDENCE as contest_SORT_ORDER,
        candidates.CANDIDATE_CODE as DB_CANDIDATE_CAN_CODE,
        candidates.NAME_ON_BALLOT as DB_CANDIDATE_NAMEONBALLOT,
        candidates.MANUAL_ORDER as DB_CANDIDATE_SORT_ORDER,
        candidates.POLITICAL_ORG_CODE as DB_CANDIDATE_NOMINATEDBY,
        political_organizations.INITIALS as DB_PARTY_SHORT_NAME,
        political_organizations.POLITICAL_ORG_NAME as DB_PARTY_NAME_PARTY,
        precinct_established.ESTABLISHED_CODE as DB_PRECINCT_ESTABLISHED_CODE
    FROM
        region
    JOIN
        polling_centers
    ON
        region.REGION_CODE = polling_centers.REGION_CODE
    JOIN
        voting_device
    ON
        polling_centers.VOTING_CENTER_CODE = voting_device.VOTING_CENTER_CODE
    CROSS JOIN
        polling_district
    JOIN
        precinct_established
    ON
        precinct_established.PRECINCT_CODE = voting_device.VOTING_DEVICE_CODE
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
    LEFT JOIN
        political_organizations
    ON
        political_organizations.POLITICAL_ORG_CODE = candidates.POLITICAL_ORG_CODE
    WHERE
        precinct_established.PRECINCT_CODE IN ({precinct_ids_str}) AND
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

def get_sbei_username(user, barangay_id):
    return f"sbei-{barangay_id}-{user['ROLE']}"

def generate_election_event(excel_data, base_context, miru_data):
    election_event_id = generate_uuid()
    miru_event = list(miru_data.values())[0]

    sbei_users = []
    sbei_users_with_permission_labels = []

    for precinct_id in miru_data.keys():
        precinct = miru_data[precinct_id]
        region = next((e for e in precinct["REGIONS"] if e["TYPE"] == "Barangay"), None)
        if not region:
            raise "Can't find post/Barangay in precinct {precinct_id}"
        barangay_id = region["ID"]
        miru_election_id = list(precinct["CONTESTS"].values())[0]["ELECTION_ID"]
        election_permission_label = next((e["permission_label"] for e in excel_data["elections"] if e["precinct_id"] == precinct_id), None)
        for user in precinct["USERS"]:
            username = get_sbei_username(user, barangay_id)
            new_user = {
                "username": username,
                "miru_id": user["ID"],
                "miru_role": user["ROLE"],
                "miru_name": user["NAME"],
                "miru_election_id": miru_election_id,
            }
            sbei_users.append(new_user)
            sbei_users_with_permission_labels.append({
                "permission_label": election_permission_label,
                "username": username,
                "miru_id": user["ID"],
                "miru_role": user["ROLE"],
                "miru_name": user["NAME"],
                "miru_election_id": miru_election_id
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
    print(election_event_context)
    return json.loads(render_template(election_event_template, election_event_context)), election_event_id, sbei_users_with_permission_labels


# "OSAKA PCG" -> "Osaka PCG"
def get_embassy(embassy):
    # Split the input string into words
    words = embassy.split()
    
    # Capitalize each word, and handle the last word conditionally
    formatted_words = [word.title() for word in words[:-1]]
    last_word = words[-1].upper() if len(words[-1]) <= 3 else words[-1].title()
    
    # Combine the formatted words with the conditionally formatted last word
    formatted_words.append(last_word)
    
    # Join the words into a single string
    return " ".join(formatted_words)


def get_country_from_area_embassy(area, embassy):
    # "PEOPLES REPUBLIC OF BANGLADESH" -> "Bangladesh"
    country = area.split()[-1].capitalize()
    return f"{country}/{embassy}"

def create_scheduled_events_file(final_json):
    scheduled_events = final_json['scheduled_events']
    try:
        # Create a zip file to store the CSV files
        election_event_id = final_json["election_event"]["id"]
        zip_filename = f"output/election-event.zip"
        events_array = []
        
        for event in scheduled_events:
            # Create a CSV file for each scheduled event
            csv_data = {
                "task_id": event["task_id"],
                "tenant_id": event["tenant_id"],
                "election_event_id": election_event_id,
                "created_at": event["created_at"],
                "stopped_at": event["stopped_at"],
                "archived_at": event["archived_at"],
                "labels": event["labels"],
                "annotations": event["annotations"],
                "event_processor": event["event_processor"],
                "cron_config": event["cron_config"],
                "event_payload": event["event_payload"]
            }
            events_array.append(csv_data)


        with zipfile.ZipFile(zip_filename, 'w', zipfile.ZIP_DEFLATED) as zipf:
            ###### Add scheduled events
            # Convert to JSON string
            json_str = json.dumps(events_array)
            
            # Create an in-memory file-like object
            csv_buffer = io.StringIO()
            csv_buffer.write(json_str)
            
            # Add the CSV file to the zip archive with a unique name
            filename = f"export_scheduled_events-{election_event_id}.csv"
            zipf.writestr(filename, csv_buffer.getvalue())
            csv_buffer.close()

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


def create_admins_file(sbei_users):
    # Data to be written to the CSV file
    csv_data = [
        [
            "enabled","first_name","username","permission_labels","password","group_name"
            #true,Eduardo,admin2,BANGKOK|DHAKA,admin2,admin
        ]
    ]
    users_map = {}
    for user in sbei_users:
        username = user["username"]
        if not username in users_map:
            users_map[username] = []
        permission_label = user["permission_label"] 
        if permission_label:
            users_map[username].append(permission_label)
    
    for key_username in users_map.keys():
        permission_labels = users_map[key_username]
        csv_data.append([
            True,
            key_username,
            key_username,
            "|".join(permission_labels),
            key_username,
            "sbei"
        ])

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
        

def gen_keycloak_context(results):

    print(f"generating keycloak context")
    country_set = set()
    embassy_set = set()

    for row in results:
        if not row["DB_ALLMUN_AREA_NAME"]:
            continue
        country_set.add("\\\"" + row["DB_ALLMUN_AREA_NAME"] + "\\\"")
        if not row["allbgy_AREANAME"]:
            continue
        embassy_set.add("\\\"" + row["DB_ALLMUN_AREA_NAME"] + "/" + row["allbgy_AREANAME"] + "\\\"")

    keycloak_context = {
        "embassy_list": "[" + ",".join(embassy_set) + "]",
        "country_list": "[" + ",".join(country_set) + "]"
    }
    return keycloak_context

def gen_tree(excel_data, results, miru_data):
    elections_object = {"elections": []}

    ccs_servers = {}

    # areas
    areas = {}
    for row in results:
        area_name = row["DB_ALLMUN_AREA_NAME"].strip()

        # the area
        if area_name in areas:
            continue

        precinct_id = row["DB_TRANS_SOURCE_ID"]
        if precinct_id not in miru_data:
            raise Exception(f"precinct with 'id' = {precinct_id} not found in miru acf")
        miru_precinct = miru_data[precinct_id]

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
            "description" :row["DB_POLLING_CENTER_POLLING_PLACE"],
            "source_id": row["DB_TRANS_SOURCE_ID"],
            "dest_id": row["trans_route_TRANS_DEST_ID"],
            **base_context,
            "miru": {
                "ccs_servers": ccs_servers_str,
                "sbei_ids": sbei_ids_str
            }
        }
        areas[area_name] = area

    for (idx, row) in enumerate(results):
        print(f"processing row {idx}")
        # Find or create the election object
        row_election_post = row["DB_POLLING_CENTER_POLLING_PLACE"]
        row_precinct_id = row["DB_TRANS_SOURCE_ID"]
        election = next((e for e in elections_object["elections"] if e["precinct_id"] == row_precinct_id), None)
        election_context = next((
            c for c in excel_data["elections"] 
            if str(c["precinct_id"]) == row_precinct_id
        ), None)

        election_context["precinct_id"] = str(election_context["precinct_id"])

        if not election_context:
            raise Exception(f"election with 'precinct_id' = {row_precinct_id} not found in excel")
        
        precinct_id = row["DB_TRANS_SOURCE_ID"]
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
                "election_name": election_context["name"],
                "contests": [],
                "scheduled_events": [],
                "miru": {
                    "election_id": miru_contest["ELECTION_ID"],
                    "name": miru_contest["NAME_ABBR"],
                    "post": row_election_post,
                    "geographical_region": miru_precinct["REGION"],
                    "precinct_code": row["DB_PRECINCT_ESTABLISHED_CODE"],
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
                "candidates": [],
                "areas": []
            }
            election["contests"].append(contest)

        # Add the candidate to the contest
        candidate_name = row["DB_CANDIDATE_NAMEONBALLOT"]
        candidate_id = row["DB_CANDIDATE_CAN_CODE"]
        if not candidate_id in miru_precinct["CANDIDATES"]:
            raise Exception(f"candidate with 'id' = {candidate_id} and precinct = {precinct_id} not found in miru acf")
        miru_candidate = miru_precinct["CANDIDATES"][candidate_id]

        candidate = {
            "code": candidate_id,
            "name_on_ballot": candidate_name,
            "party_short_name": row["DB_PARTY_SHORT_NAME"],
            "party_name": row["DB_PARTY_NAME_PARTY"],
            **base_context,
            "miru": {
                "candidate_affiliation_id": miru_candidate["PARTY_ID"],
                "candidate_affiliation_party": miru_candidate["PARTY_NAME"],
                "candidate_affiliation_registered_name": miru_candidate["PARTY_NAME_ABBR"],
            },
        }
        found_candidate = next((
            c for c in contest["candidates"]
            if c["code"] == candidate["code"] and
            c["name_on_ballot"] == candidate["name_on_ballot"] and
            c["party_name"] == candidate["party_name"]),
        None)

        if found_candidate is None:
            contest["candidates"].append(candidate)

        # Add the area to the contest if it hasn't been added already
        area_name = row["DB_ALLMUN_AREA_NAME"]
        if area_name not in contest["areas"]:
            contest["areas"].append(area_name)

    # test elections
    test_elections =  copy.deepcopy(elections_object["elections"])
    for election in test_elections:
        election["name"] = "Test Voting"
        election["alias"] = "Test Voting"

    elections_object["elections"].extend(test_elections)

    # scheduled events
    for election in elections_object["elections"]:
        election_scheduled_events = [
            scheduled_event
            for scheduled_event
            in excel_data["scheduled_events"] 
            if scheduled_event["election_alias"] == election["alias"]
        ]
        election["scheduled_events"] = election_scheduled_events

    return elections_object, areas

def replace_placeholder_database(election_tree, areas_dict, election_event_id, keycloak_context, miru_data):
    area_contests = []
    area_contexts_dict = {}
    areas = []
    candidates = []
    contests = []
    elections = []
    scheduled_events = []

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
        template_render = render_template(election_template, election_context)
        elections.append(json.loads(template_render))

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
            scheduled_events.append(json.loads(render_template(scheduled_event_template, scheduled_event_context)))


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
            contests.append(json.loads(render_template(contest_template, contest_context)))

            for candidate in contest["candidates"]:
                candidate_context = {
                    **candidate,
                    "UUID": generate_uuid(),
                    "tenant_id": base_config["tenant_id"],
                    "election_event_id": election_event_id,
                    "contest_id": contest_context["UUID"],
                    "DB_CANDIDATE_NAMEONBALLOT": candidate["name_on_ballot"]
                }

                print(f"rendering candidate {candidate['name_on_ballot']}")
                candidates.append(json.loads(render_template(candidate_template, candidate_context)))

            for area_name in contest["areas"]:
                area_name = area_name.strip()
                if area_name not in areas_dict:
                    raise Exception(f"area not found {area_name}")
                area = areas_dict[area_name]

                if area_name not in area_contexts_dict:
                    area_context = {
                        **area,
                        "UUID": generate_uuid(),
                        "tenant_id": base_config["tenant_id"],
                        "election_event_id": election_event_id,
                        "DB_TRANS_SOURCE_ID": area["source_id"],
                        "DB_ALLMUN_AREA_NAME": area["name"],
                        "DB_POLLING_CENTER_POLLING_PLACE":area["description"]
                    }
                    area_contexts_dict[area_name] = area_context

                    print(f"rendering area {area['name']}")
                    rendered_area_template = render_template(area_template, area_context)
                    areas.append(json.loads(rendered_area_template))
                else:
                    area_context = area_contexts_dict[area_name]

                area_contest_context = {
                    "UUID": generate_uuid(),
                    "area_id": area_context["UUID"],
                    "contest_id": contest_context["UUID"]
                }

                print(f"rendering area_contest area: '{area['name']}', contest: '{contest['name']}'")

                area_contests.append(json.loads(render_template(area_contest_template, area_contest_context)))

    return areas, candidates, contests, area_contests, elections, keycloak, scheduled_events



def parse_table_sheet(
    sheet,
    required_keys=[],
    allowed_keys=[],
    map_f=lambda value: value
):
    '''
    Reads a CSV table and returns it as a list of dict items.
    '''
    def check_required_keys(header_values, required_keys):
        '''
        Check that each required_key pattern appears in header_values
        '''
        matched_patterns = set()
        for key in header_values:
            for pattern in required_keys:
                if re.match(pattern, key):
                    matched_patterns.add(pattern)
                    break
        assert(len(matched_patterns) == len(required_keys))

    def check_allowed_keys(header_values, allowed_keys):
        allowed_keys += [
            r"^name$",
            r"^alias$",
            r"^annotations\.[_a-zA-Z0-9]+",
        ]
        matched_patterns = set()
        for key in header_values:
            found = False
            for pattern in allowed_keys:
                if re.match(pattern, key):
                    matched_patterns.add(pattern)
                    found = True
                    break
            if not found:
                raise Exception(f"header {key} not allowed")

    def parse_line(header_values, line_values):
        '''
        Once all keys are validated, let's parse them in the desired structure
        '''
        parsed_object = dict()
        for (key, value) in zip(header_values, line_values):
            split_key = key.split('.')
            subelement = parsed_object
            for split_key_index, split_key_item in enumerate(split_key):
                # if it's not last
                if split_key_index == len(split_key) - 1:
                    if isinstance(value, float):
                        subelement[split_key_item] = int(value)
                    else:
                        subelement[split_key_item] = value
                else:
                    if split_key_item not in subelement:
                        subelement[split_key_item] = dict()
                    subelement = subelement[split_key_item]

        return map_f(parsed_object)

    def sanitize_values(values):
        return [
            sanitize_value(value)
            for value in values
        ]

    def sanitize_value(value):
        return value.strip() if isinstance(value, str) else value

    # Get header and check required and allowed keys
    header_values = None
    ret_data = []
    for row in sheet.values:
        sanitized_row = sanitize_values(row)
        if not header_values:
            header_values = [
                value
                for value in sanitized_row
                if value is not None
            ]
            check_required_keys(header_values, required_keys)
            check_allowed_keys(header_values, allowed_keys)
        else:
            ret_data.append(
                parse_line(header_values, sanitized_row)
            )

    return ret_data

def parse_election_event(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^logo_url$"
        ],
        allowed_keys=[
            "^logo_url$"
        ]
    )
    return data[0]

def parse_elections(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^precinct_id$",
            "^description$"
        ],
        allowed_keys=[
            "^precinct_id$",
            "^description$",
            "^permission_label$"
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

def parse_excel(excel_path):
    '''
    Parse all input files specified in the config file into their respective
    data structures.
    '''
    electoral_data = openpyxl.load_workbook(excel_path)

    return dict(
        election_event = parse_election_event(electoral_data['ElectionEvent']),
        elections = parse_elections(electoral_data['Elections']),
        scheduled_events = parse_scheduled_events(electoral_data['ScheduledEvents']),
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

def read_inspector_pwds(miru_path):
    inspector_pwds = {}
    file_path = os.path.join(miru_path, "pass.txt")
    with open(file_path, 'r') as file:
        for line in file:
            # Split the line into parts and extract the required information
            parts = line.split()
            if len(parts) >= 2:
                id_value = parts[0]
                code_value = parts[-1] 
                inspector_pwds[id_value] = code_value
    return inspector_pwds

def read_miru_data(acf_path, cf_id, script_dir):
    acf_path = os.path.join(miru_path, f'ACF-0-{cf_id}')
    inspector_pwds = read_inspector_pwds(miru_path)
    data = {}
    folders = list_folders(acf_path)
    for precinct_id in folders:
        precinct_file = read_json_file(os.path.join(acf_path, precinct_id, 'precinct.acf'))
        security_file = read_json_file(os.path.join(acf_path, precinct_id, 'security.acf'))
        server_file = read_json_file(os.path.join(acf_path, precinct_id, 'server.acf'))
        user_file = read_json_file(os.path.join(acf_path, precinct_id, 'user.acf'))

        servers = index_by(server_file["SERVERS"], "ID")
        security = index_by(security_file["CERTIFICATES"], "ID")
        keystore_path = os.path.join(acf_path, precinct_id, 'keystore.bks')
        
        if not args.only_voters:
            print(f"Reading keys for precint {precinct_id}")

            for user in user_file["USERS"]:
                if "07" == user["ROLE"]:
                    continue
                user_id = user["ID"]
                if not user_id in inspector_pwds:
                    raise f"sbei user {user_id} not found"
                
                password = inspector_pwds[user_id]
                command = f"""keytool -importkeystore \
                    -srckeystore {keystore_path} \
                    -srcstoretype BKS \
                    -srcstorepass '' \
                    -srckeypass '{password}' \
                    -srcalias eb_{user_id} \
                    -destkeystore output/sbei_{user_id}.p12 \
                    -deststoretype PKCS12 \
                    -deststorepass '{password}' \
                    -destkeypass '{password}' \
                    -destalias eb_91070001-01 \
                    -providerpath bcprov.jar \
                    -provider org.bouncycastle.jce.provider.BouncyCastleProvider"""
                run_command(command, script_dir)
        
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
            run_command(command, script_dir)
            
            alias_pubkey = read_text_file(alias_path)
            server["PUBLIC_KEY"] = alias_pubkey

        election = precinct_file["ELECTIONS"][0]
        region = next((e for e in precinct_file["REGIONS"] if e["TYPE"] == "Province"), None)

        precinct_data = {
            "EVENT_ID": election["EVENT_ID"],
            "EVENT_NAME": election["NAME"],
            "CONTESTS": index_by(precinct_file["CONTESTS"], "ID"),
            "CANDIDATES": index_by(precinct_file["CANDIDATES"], "ID"),
            "REGIONS": precinct_file["REGIONS"],
            "REGION": region["NAME"],
            "SERVERS": servers,
            "USERS": user_file["USERS"],
        }
        data[precinct_id] = precinct_data

    return data

def find_acf_id(folder_path: str) -> str:
    # Regular expression to match "ACF-0-" followed by any sequence of digits or letters
    pattern = r"ACF-0-(.+)"
    
    # Iterate through the contents of the folder
    for item in os.listdir(folder_path):
        item_path = os.path.join(folder_path, item)
        
        # Check if the item is a directory and matches the pattern
        if os.path.isdir(item_path):
            match = re.match(pattern, item)
            if match:
                # Return whatever is after "ACF-0-" as a string
                return match.group(1)
    
    # Return None if no matching folder is found
    raise 'Path not found ACF-0-<number>'

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


# Step 2: Set up argument parsing
parser = argparse.ArgumentParser(description="Process a MYSQL COMELEC DUMP .sql file, and generate the electionconfig.json")
parser.add_argument('miru', type=str, help='Base name of the Miru files') # example: 'import-data/CCF-0-20241021'
parser.add_argument('excel', type=str, help='Excel config (with .xlsx extension)')
parser.add_argument('--voters', type=str, metavar='VOTERS_FILE_PATH', help='Create a voters file if this flag is set')
parser.add_argument('--only-voters', type=str, metavar='VOTERS_FILE_PATH', help='Only create a voters file if this flag is set')

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

# Step 5: Convert the csv to sql
sql_output_path = 'data/miru.sql'
sqlite_output_path = 'data/db_sqlite_miru.db'
remove_file_if_exists(sql_output_path)
remove_file_if_exists(sqlite_output_path)
cf_id = find_acf_id(miru_path)
miru_data = read_miru_data(miru_path, cf_id, script_dir)
render_sql(miru_path + f'/CCF-0-{cf_id}/election_data/', sql_output_path, voters_path)


# Step 6: Convert MySQL dump to SQLite
command = f"chmod +x mysql2sqlite && ./mysql2sqlite {sql_output_path} | sqlite3 {sqlite_output_path}"

# Log the constructed command
logging.debug(f"Constructed command: {command}")

run_command(command, script_dir)

# Step 7: Read Excel
excel_data = parse_excel(excel_path)

# Step 8: Read the sqlite db
results = get_data(sqlite_output_path, excel_data)


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

    logging.info("Loaded all templates successfully.")
except FileNotFoundError as e:
    logging.exception(f"Template file not found: {e}")
except Exception as e:
    logging.exception("An error occurred while loading templates.")


# Example of how to use the function and see the result

if voters_path:
    create_voters_file(sqlite_output_path)

if args.only_voters:
    print("Only voters, exiting the script.")
    sys.exit()

election_tree, areas_dict = gen_tree(excel_data, results, miru_data)
keycloak_context = gen_keycloak_context(results)
election_event, election_event_id, sbei_users = generate_election_event(excel_data, base_context, miru_data)
create_admins_file(sbei_users)

areas, candidates, contests, area_contests, elections, keycloak, scheduled_events = replace_placeholder_database(election_tree, areas_dict, election_event_id, keycloak_context, miru_data)

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
    "reports": []
}



# Step 14: Save final ZIP to a file
try:
    # Create the scheduled events zip file after generating the final JSON
    create_scheduled_events_file(final_json)
    logging.info("Final ZIP generated and saved successfully.")
except Exception as e:
    logging.exception("An error occurred while saving the final JSON.")
