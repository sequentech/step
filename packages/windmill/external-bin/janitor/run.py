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

# Step 2: Set up argument parsing
parser = argparse.ArgumentParser(description="Process a MYSQL COMELEC DUMP .sql file, and generate the electionconfig.json")
parser.add_argument('filename', type=str, help='Base name of the SQL file (with .sql extension)')
parser.add_argument('excel', type=str, help='Excel config (with .xlsx extension)')
parser.add_argument('--voters', action='store_true', help='Create a voters file if this flag is set')
parser.add_argument('--only-voters', action='store_true', help='Only create a voters file if this flag is set')

# Step 3: Parse the arguments
args = parser.parse_args()

# Step 4: Use the filename argument in your command
filename = args.filename
logging.debug(f"Filename received: {filename}")

excel_path = args.excel
logging.debug(f"Excel received: {excel_path}")

# Step 5: Determine the script's directory to use as cwd
script_dir = os.path.dirname(os.path.abspath(__file__))



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
            "^description$",
            "^logo_url$"
        ],
        allowed_keys=[
            "^description$",
            "^logo_url$"
        ]
    )
    return data[0]

def parse_elections(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            r"^election_post$",
            "^description$"
        ],
        allowed_keys=[
            r"^election_post$",
            "^description$"
        ]
    )
    return data

def parse_contests(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^db_contest_name$",
            "^election_post$"
        ],
        allowed_keys=[
            "^db_contest_name$",
            "^election_post$"
        ]
    )
    return data

def parse_candidates(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^db_contest_name$",
            "^election_post$"
        ],
        allowed_keys=[
            "^db_contest_name$",
            "^election_post$"
        ]
    )
    return data

def parse_areas(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^description$"
        ],
        allowed_keys=[
            "^description$"
        ]
    )
    return data

def parse_ccs_servers(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^tag$",
            "^address$",
            "^public_key$",
            "^send_logs$"
        ],
        allowed_keys=[
            "^tag$",
            "^address$",
            "^public_key$",
            "^send_logs$"
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
        areas = parse_areas(electoral_data['Areas']),
        ccs_servers = parse_ccs_servers(electoral_data['CcsServers']),
        scheduled_events = parse_scheduled_events(electoral_data['ScheduledEvents']),
    )

# Step 5.1: Read Excel
excel_data = parse_excel(excel_path)

# Step 6: Removing Candidate Blob and convert MySQL dump to SQLite
command = f"chmod +x removecandidatesblob.py mysql2sqlite && python3 ./removecandidatesblob.py {filename} data/db_mysql_no_blob.sql && ./mysql2sqlite data/db_mysql_no_blob.sql | sqlite3 data/db_sqlite.db"

# Log the constructed command
logging.debug(f"Constructed command: {command}")

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

# Step 7: Connect to SQLite database
dbfile = 'data/db_sqlite.db'
try:
    conn = sqlite3.connect(dbfile)
    conn.row_factory = sqlite3.Row  # This allows rows to be accessed like dictionaries
    cursor = conn.cursor()
    logging.info(f"Connected to SQLite database at {dbfile}.")
except sqlite3.Error as e:
    logging.exception(f"Failed to connect to SQLite database: {e}")

# Step 8: Query SQLite database to check tables
try:
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
    tables = cursor.fetchall()
    logging.debug(f"Tables in the database: {tables}")
except sqlite3.Error as e:
    logging.exception(f"Failed to retrieve tables from SQLite database: {e}")

# Step 9: Load base configuration
try:
    with open('config/baseConfig.json', 'r') as file:
        base_config = json.load(file)
        logging.info("Loaded base configuration.")
except FileNotFoundError:
    logging.exception("Base configuration file not found.")
except json.JSONDecodeError:
    logging.exception("Failed to parse base configuration file.")

# Step 10: Generate UUIDs and get the current timestamp
def generate_uuid():
    return str(uuid.uuid4())
logging.debug(f"Generated UUID: {generate_uuid()}")

current_timestamp = datetime.now(timezone.utc).isoformat()
logging.debug(f"Current timestamp: {current_timestamp}")

# Step 11: Retrieve data from SQLite database
def get_sqlite_data(query):
    try:
        cursor.execute(query)
        result = cursor.fetchall()
        logging.debug(f"Query executed: {query}, Result: {result}")
        return result
    except sqlite3.Error as e:
        logging.exception(f"Failed to execute query: {query}")
        return []

# Step 12: Compile and render templates using pybars3
compiler = Compiler()

def render_template(template_str, context):
    template = compiler.compile(template_str)
    return template(context)

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

def get_voters():
    query = """SELECT 
        voter_demo_ovcs.FIRSTNAME as voter_FIRSTNAME,
        voter_demo_ovcs.LASTNAME as voter_LASTNAME,
        voter_demo_ovcs.DATEOFBIRTH as voter_DATEOFBIRTH,
        CASE 
            WHEN allbgy.AREANAME != polling_centers.POLLING_PLACE 
            THEN allbgy.AREANAME 
            ELSE allmun.AREANAME 
        END as DB_ALLMUN_AREA_NAME,
        polling_centers.POLLING_PLACE as DB_POLLING_CENTER_POLLING_PLACE
    FROM
        voter_demo_ovcs
    LEFT JOIN
        pop ON pop.PRECINCT = voter_demo_ovcs.PRECINCT
    LEFT JOIN
        allbgy ON pop.CLUSTERPOLLCENTER = allbgy.ID_BARANGAY
    LEFT JOIN 
        allmun ON (pop.PROV_CODE || pop.MUN_CODE) = allmun.ID_CITY
    LEFT JOIN 
        polling_centers ON polling_centers.ID = pop.POLLCENTER_CODE;
    """
    return get_sqlite_data(query)

def get_data():
    query = """SELECT 
    pop.POLLCENTER_CODE as pop_POLLCENTER_CODE,
    allbgy.ID_BARANGAY as allbgy_ID_BARANGAY,
    allbgy.AREANAME as allbgy_AREANAME,
    allmun.ID_CITY as allmun_ID_CITY,
    CASE 
        WHEN allbgy.AREANAME != polling_centers.POLLING_PLACE 
        THEN allbgy.AREANAME 
        ELSE allmun.AREANAME 
    END as DB_ALLMUN_AREA_NAME,
    polling_centers.POLLING_PLACE as DB_POLLING_CENTER_POLLING_PLACE,
    trans_route.TRANS_SOURCE_ID as DB_TRANS_SOURCE_ID,
    trans_route.TRANS_DEST_ID as trans_route_TRANS_DEST_ID,
    seat.CONTEST as DB_CONTEST_NAME,
    seat.ELIGIBLEAMOUNT as DB_RACE_ELIGIBLEAMOUNT,
    seat.DISTRICTCODE as DB_SEAT_DISTRICTCODE,
    contest.POSTCODE as contest_POSTCODE,
    contest.SORT_ORDER as contest_SORT_ORDER,
    candidate.CAND_CODE as DB_CANDIDATE_CAN_CODE,
    candidate.NAMEONBALLOT as DB_CANDIDATE_NAMEONBALLOT,
    candidate.NOMINATEDBY as DB_CANDIDATE_NOMINATEDBY,
    party_list.SHORT_NAME as DB_PARTY_SHORT_NAME,
    party_list.NAME_PARTY as DB_PARTY_NAME_PARTY
FROM 
    pop 
JOIN 
    allbgy ON pop.CLUSTERPOLLCENTER = allbgy.ID_BARANGAY 
LEFT JOIN 
    allmun ON (pop.PROV_CODE || pop.MUN_CODE) = allmun.ID_CITY
LEFT JOIN 
    polling_centers ON polling_centers.ID = pop.POLLCENTER_CODE
LEFT JOIN 
    trans_route ON (pop.PROV_CODE || pop.MUN_CODE || '0' || pop.BRGY_CODE) = trans_route.TRANS_SOURCE_ID
CROSS JOIN 
    seat
LEFT JOIN 
    contest ON seat.CONTEST = contest.POSTNAME
JOIN 
    candidate ON contest.POSTCODE = candidate.POST_CODE
LEFT JOIN  
    party_list ON candidate.NOMINATEDBY = party_list.CODE_PARTY;"""
    return get_sqlite_data(query)

base_context = {
    "tenant_id": base_config["tenant_id"],
    "current_timestamp": current_timestamp
}

def generate_election_event(excel_data):
    election_event_id = generate_uuid()
    election_event_context = {
        "UUID": election_event_id,
        **base_context,
        **excel_data["election_event"]

    }
    print(election_event_context)
    return json.loads(render_template(election_event_template, election_event_context)), election_event_id


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

def create_voters_file():
    voters_sql = get_voters()
    # Data to be written to the CSV file
    csv_data = [
        [
             "enabled", "first_name", "last_name", "birthDate", "area_name", "embassy", "country", "group_name"
        ]
    ]
    for row in voters_sql:
        embassy = get_embassy(row["DB_POLLING_CENTER_POLLING_PLACE"])
        csv_data.append([
            "TRUE",
            row["voter_FIRSTNAME"].title(),
            row["voter_LASTNAME"].title(),
            row["voter_DATEOFBIRTH"],
            row["DB_ALLMUN_AREA_NAME"],
            embassy,
            get_country_from_area_embassy(row["DB_ALLMUN_AREA_NAME"], embassy),
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

def gen_tree(excel_data, results):
    elections_object = {"elections": []}

    ccs_servers = {}
    for ccs_server in excel_data["ccs_servers"]:
        if not ccs_server["tag"]:
            continue
        json_server = json.dumps({
            "send_logs": "TRUE" == ccs_server["send_logs"],
            "name": ccs_server["name"],
            "tag": str(int(ccs_server["tag"])),
            "address": ccs_server["address"],
            "public_key_pem": ccs_server["public_key"]
        })
        ccs_servers[str(int(ccs_server["tag"]))] = json_server
    
    # areas
    areas = {}
    for row in results:
        area_name = row["DB_ALLMUN_AREA_NAME"]

        # the area
        if area_name in areas:
            continue
        area_context = next((
            c for c in excel_data["areas"] 
            if c["name"] == area_name
        ), None)

        if not area_context:
            raise Exception(f"area with 'name' = {area_name} not found in excel")

        ccs_server_tags = str(area_context["annotations"]["miru_ccs_server_tags"]).split(",")
        ccs_server_tags = [str(int(float(i))) for i in ccs_server_tags]

        found_servers = [
            ccs_servers[tag].replace('"', '\\"')
            for tag in ccs_server_tags
            if tag in ccs_servers
        ]
        miru_trustee_users = area_context["annotations"]["miru_trustee_servers"].split(",")
        miru_trustee_users = [('"' + server + '"') for server in miru_trustee_users]
        miru_trustee_users = ",".join(miru_trustee_users)
        area_context["annotations"]["miru_ccs_servers"] = "[" + ",".join(found_servers) + "]"
        area_context["annotations"]["miru_trustee_users"] = "[" + miru_trustee_users.replace('"', '\\"') + "]"

        area = {
            "name": area_name,
            "description" :row["DB_POLLING_CENTER_POLLING_PLACE"],
            "source_id": row["DB_TRANS_SOURCE_ID"],
            "dest_id": row["trans_route_TRANS_DEST_ID"],
            **base_context,
            **area_context
        }
        areas[area_name] = area

    for (idx, row) in enumerate(results):
        print(f"processing row {idx}")
        # Find or create the election object
        row_election_post = row["DB_POLLING_CENTER_POLLING_PLACE"]
        election = next((e for e in elections_object["elections"] if e["election_post"] == row_election_post), None)
        election_context = next((
            c for c in excel_data["elections"] 
            if c["election_post"] == row_election_post
        ), None)

        if not election_context:
            raise Exception(f"election with 'election_post' = {row_election_post} not found in excel")
        
        if not election:
            # If the election does not exist, create it
            election = {
                "election_post": row_election_post,
                "election_name": election_context["name"],
                "contests": [],
                "scheduled_events": [],
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
                "postcode": row["contest_POSTCODE"],
                "sort_order": row["contest_SORT_ORDER"],
                "candidates": [],
                "areas": []
            }
            election["contests"].append(contest)

        # Add the candidate to the contest
        candidate_name = row["DB_CANDIDATE_NAMEONBALLOT"]

        candidate = {
            "code": row["DB_CANDIDATE_CAN_CODE"],
            "name_on_ballot": candidate_name,
            "nominated_by": row["DB_CANDIDATE_NOMINATEDBY"],
            "party_short_name": row["DB_PARTY_SHORT_NAME"],
            "party_name": row["DB_PARTY_NAME_PARTY"],
            **base_context,
            "annotations": {
                "miru_candidate_affiliation_id": row["DB_CANDIDATE_NOMINATEDBY"] if row["DB_CANDIDATE_NOMINATEDBY"] else " ",
                "miru_candidate_affiliation_party": row["DB_CANDIDATE_NOMINATEDBY"] if row["DB_CANDIDATE_NOMINATEDBY"] else "NULL",
                "miru_candidate_affiliation_registered_name": row["DB_CANDIDATE_NOMINATEDBY"] if row["DB_CANDIDATE_NOMINATEDBY"] else "NULL",
            }
        }
        found_candidate = next((
            c for c in contest["candidates"]
            if c["code"] == candidate["code"] and
            c["name_on_ballot"] == candidate["name_on_ballot"] and
            c["nominated_by"] == candidate["nominated_by"] and
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

def replace_placeholder_database(election_tree, areas_dict, election_event_id, keycloak_context):
    area_contests = []
    area_contexts_dict = {}
    areas = []
    candidates = []
    contests = []
    elections = []
    scheduled_events = []

    print(f"rendering keycloak")
    keycloak = json.loads(render_template(keycloak_template, keycloak_context))
    

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
        elections.append(json.loads(render_template(election_template, election_context)))

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
                if area_name not in areas_dict:
                    breakpoint()
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
                    areas.append(json.loads(render_template(area_template, area_context)))
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

# Example of how to use the function and see the result

if args.voters or args.only_voters:
    create_voters_file()

if args.only_voters:
    print("Only voters, exiting the script.")
    sys.exit()

results = get_data()
election_tree, areas_dict = gen_tree(excel_data, results)
keycloak_context = gen_keycloak_context(results)
election_event, election_event_id = generate_election_event(excel_data)

areas, candidates, contests, area_contests, elections, keycloak, scheduled_events = replace_placeholder_database(election_tree, areas_dict, election_event_id, keycloak_context)

final_json = {
    "tenant_id": base_config["tenant_id"],
    "keycloak_event_realm": keycloak,  # Add appropriate value or leave it as is
    "election_event": election_event,  # Include the generated election event
    "elections": elections,  # Include the election objects
    "contests": contests,  # Include the contest objects
    "candidates":candidates, # Include the candidate objects
    "areas": areas,  # Include the area objects
    "area_contests": area_contests,  # Include the area-contest relationships
    "scheduled_events": scheduled_events,
    "reports": []
}

# Step 14: Save final JSON to a file
try:
    with open('output/election_config.json', 'w') as file:
        json.dump(final_json, file, indent=4)
    logging.info("Final JSON generated and saved successfully.")
except Exception as e:
    logging.exception("An error occurred while saving the final JSON.")

# Close the SQLite connection
try:
    conn.close()
    logging.info("SQLite connection closed.")
except sqlite3.Error as e:
    logging.exception(f"Failed to close SQLite connection: {e}")

logging.info("Script finished.")
