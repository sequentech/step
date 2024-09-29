import json
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
            r"^Metadata\.Name$",
            r"^Metadata\.Labels\[\d+\]\.Key$",
            r"^Metadata\.Labels\[\d+\]\.Value$",
            r"^Metadata\.Template$",
            r"^Extra\.[a-zA-Z0-9]+$"
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
                    if '[' not in split_key_item:
                        subelement[split_key_item] = value
                    else:
                        match = re.match(
                            r"([a-zA-Z0-9]+)\[(\d+)\]$",
                            split_key_item
                        )
                        split_key_name = match.group(1)
                        split_key_subindex = int(match.group(2))
                        if split_key_name not in subelement:
                            subelement[split_key_name] = []
                        assert(
                            split_key_subindex <= len(subelement[split_key_name])
                        )
                        subelement[split_key_name].append(value)
                else:
                    if '[' not in split_key_item:
                        if split_key_item not in subelement:
                            subelement[split_key_item] = dict()
                        subelement = subelement[split_key_item]
                    else:
                        match = re.match(
                            r"([a-zA-Z0-9]+)\[(\d+)\]$",
                            split_key_item
                        )
                        split_key_name = match.group(1)
                        split_key_subindex = int(match.group(2))
                        if split_key_name not in subelement:
                            subelement[split_key_name] = [dict()]
                        assert(
                            split_key_subindex <= len(subelement[split_key_name])
                        )
                        subelement = subelement[split_key_name][split_key_subindex]

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
            r"^name$",
            "^description$",
            "^miru election event id$",
            "^miru election event name$",
            "^logo url$"
        ],
        allowed_keys=[
            r"^name$",
            "^description$",
            "^miru election event id$",
            "^miru election event name$",
            "^logo url$"
        ]
    )
    return data[0]

def parse_elections(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            r"^name$",
            "^alias$",
            "^description$",
            "^miru election id$",
            "^miru election name$",
            "^election post$"
        ],
        allowed_keys=[
            r"^name$",
            "^alias$",
            "^description$",
            "^miru election id$",
            "^miru election name$",
            "^election post$"
        ]
    )
    return data

def parse_contests(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            r"^name$",
            "^alias$",
            "^election name$",
            "^miru contest id$",
            "^miru contest name$"
        ],
        allowed_keys=[
            r"^name$",
            "^alias$",
            "^election name$",
            "^miru contest id$",
            "^miru contest name$"
        ]
    )
    return data

def parse_candidates(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            r"^name$",
            "^alias$",
            "^contest name$",
            "^election name$",
            "^miru candidate id$",
            "^miru candidate name$",
            "^miru candidate setting$",
            "^miru candidate affiliation id$",
            "^miru candidate affiliation party$",
            "^miru candidate affiliation registered name$"
        ],
        allowed_keys=[
            r"^name$",
            "^alias$",
            "^contest name$",
            "^election name$",
            "^miru candidate id$",
            "^miru candidate name$",
            "^miru candidate setting$",
            "^miru candidate affiliation id$",
            "^miru candidate affiliation party$",
            "^miru candidate affiliation registered name$"
        ]
    )
    return data

def parse_areas(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            r"^name$",
            "^description$",
            "^miru area threshold$",
            "^miru area station id$",
            "^miru area trustee users$"
        ],
        allowed_keys=[
            r"^name$",
            "^description$",
            "^miru area threshold$",
            "^miru area station id$",
            "^miru area trustee users$"
        ]
    )
    return data

def parse_ccs_servers(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^name$",
            "^tag$",
            "^address$",
            "^public key$",
            "^send logs$"
        ],
        allowed_keys=[
            "^name$",
            "^tag$",
            "^address$",
            "^public key$",
            "^send logs$"
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
        contests = parse_contests(electoral_data['Contests']),
        candidates = parse_candidates(electoral_data['Candidates']),
        areas = parse_areas(electoral_data['Areas']),
        ccs_servers = parse_ccs_servers(electoral_data['CcsServers']),
    )

# Step 5.1: Read Excel
excel_data = parse_excel(excel_path)

# Step 6: Removing Candidate Blob and convert MySQL dump to SQLite
command = f"chmod +x removecandidatesblob mysql2sqlite && ./removecandidatesblob < {filename} > data/db_mysql_no_blob.sql && ./mysql2sqlite data/db_mysql_no_blob.sql | sqlite3 data/db_sqlite.db"

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

    with open('templates/css.hbs', 'r') as file:
        css_template = file.read()
        print(css_template)

    with open('templates/COMELEC/keycloack.hbs', 'r') as file:
        keycloak_template = json.load(file) 

    logging.info("Loaded all templates successfully.")
except FileNotFoundError as e:
    logging.exception(f"Template file not found: {e}")
except Exception as e:
    logging.exception("An error occurred while loading templates.")


def generate_context(excel_data):
    #excel_data
    # Step 13: Prepare context for rendering
    context = {
    #    "UUID": generate_uuid(),
        "current_timestamp": current_timestamp,
        "tenant_id": base_config["tenant_id"],
        "election_event": excel_data["election_event"]
        # "miru_election-event-id": base_config["election_event"]["miru_election-event-id"],
        # "miru_election-id": base_config["election"]["miru_election-id"],
        # "miru_election-event-name": base_config["election_event"]["miru_election-event-name"],
        # "miru_election-name": base_config["election"]["miru_election-name"],
        # "election_event_name": base_config["election_event"]["name"],
        # "election_name": base_config["election"]["name"],
        # "election_event_description": base_config["election_event"]["description"],
        # "election_event_logo_url": base_config["election_event"]["logo_url"],
        # Add other replacements as needed from SQLite queries
    }

context = generate_context(excel_data)

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
    allbgy ON (pop.PROV_CODE || pop.MUN_CODE || pop.BRGY_CODE) = allbgy.ID_BARANGAY 
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

def generate_election_event():
    election_event_id = generate_uuid()
    election_event_context = {
        "UUID": election_event_id,
        **context
#        "css": css_template

    }
    print(election_event_context)
    return json.loads(render_template(election_event_template, election_event_context)), election_event_id

def gen_tree(excel_data):
    results = get_data()
    elections_object = {"elections": []}

    for row in results:
        # Find or create the election object
        row_election_post = row["DB_POLLING_CENTER_POLLING_PLACE"]
        election = next((e for e in elections_object["elections"] if e["election_post"] == row_election_post), None)
        election_context = next((
            c for c in excel_data["elections"] 
            if c["election post"] == row_election_post
        ), None)

        if not election_context:
            raise Exception(f"election with 'election post' = {row_election_post} not found in excel")
        
        if not election:
            # If the election does not exist, create it
            election = {
                "election_post": row_election_post,
                "election_name": election_context["name"],
                "contests": [],
                **election_context
            }
            elections_object["elections"].append(election)

        # Find or create the contest object within the election
        contest_name = row["DB_CONTEST_NAME"]
        contest = next((c for c in election["contests"] if c["name"] == contest_name), None)
        contest_context = next((
            c for c in excel_data["contests"] 
            if c["name"] == contest_name and c["election name"] == election["name"]
        ), None)

        if not contest_context:
            raise Exception(f"contest with 'name' = {contest_name} and 'election name' = {election["name"]} not found in excel")
        
        if not contest:
            # If the contest does not exist, create it
            contest = {
                "name": contest_name,
                "eligible_amount": row["DB_RACE_ELIGIBLEAMOUNT"],
                "district_code": row["DB_SEAT_DISTRICTCODE"],
                "postcode": row["contest_POSTCODE"],
                "sort_order": row["contest_SORT_ORDER"],
                "candidates": [],
                "areas": [],
                **contest_context
            }
            election["contests"].append(contest)

        # Add the candidate to the contest
        candidate_name = row["DB_CANDIDATE_NAMEONBALLOT"]
        candidate_context = next((
            c for c in excel_data["candidates"] 
            if c["name"] == candidate_name and c["election name"] == election["name"] and c["contest name"] == contest["name"]
        ), None)

        if not candidate_context:
            raise Exception(f"candidate with 'name' = {candidate_name} and 'election name' = {election["name"]} and 'contest name' = {contest["name"]} not found in excel")

        candidate = {
            "code": row["DB_CANDIDATE_CAN_CODE"],
            "name_on_ballot": candidate_name,
            "nominated_by": row["DB_CANDIDATE_NOMINATEDBY"],
            "party_short_name": row["DB_PARTY_SHORT_NAME"],
            "party_name": row["DB_PARTY_NAME_PARTY"],
            **context
        }
        contest["candidates"].append(candidate)

        # Add the area to the contest if it hasn't been added already
        area_name = row["DB_ALLMUN_AREA_NAME"]
        area_context = next((
            c for c in excel_data["areas"] 
            if c["name"] == area_name
        ), None)

        if not area_context:
            raise Exception(f"area with 'name' = {area_name} not found in excel")

        ccs_servers = [
            c for c in excel_data["ccs_servers"] 
            if c["area name"] == area_name
        ]
        area_context['css servers'] = ccs_servers

        area = {
            "name": area_name,
            "description" :row["DB_POLLING_CENTER_POLLING_PLACE"],
            "source_id": row["DB_TRANS_SOURCE_ID"],
            "dest_id": row["trans_route_TRANS_DEST_ID"],
            **area_context
        }
        
        if area not in contest["areas"]:
            contest["areas"].append(area)

    return elections_object


def replace_placeholder_database(election_tree, election_event_id):
    area_contests = []
    areas = []
    candidates = []
    contests = []
    elections = []

    for election in election_tree["elections"]:
        election_id = generate_uuid()
        election_context = {
            "UUID": election_id,
            "tenant_id": base_config["tenant_id"],
            "election_event_id": election_event_id,
            "current_timestamp": current_timestamp,
            "DB_POLLING_CENTER_POLLING_PLACE": election["election_post"],
            "election_post": election["election_post"],
            "election_name": election["election_name"]
        }

        elections.append(json.loads(render_template(election_template, election_context)))

        for contest in election["contests"]:
            contest_id = generate_uuid()
            contest_context = {
                "UUID": contest_id,
                "tenant_id": base_config["tenant_id"],
                "election_event_id": election_event_id,
                "election_id": election_context["UUID"],
                "DB_CONTEST_NAME": contest["name"],
                "DB_RACE_ELIGIBLEAMOUNT": contest["eligible_amount"],
                "DB_POLLING_CENTER_POLLING_PLACE": election["election_post"],
                "current_timestamp": current_timestamp
            }

            contests.append(json.loads(render_template(contest_template, contest_context)))

            for candidate in contest["candidates"]:
                candidate_context = {
                    "UUID": generate_uuid(),
                    "tenant_id": base_config["tenant_id"],
                    "election_event_id": election_event_id,
                    "contest_id": contest_context["UUID"],
                    "DB_CANDIDATE_NAMEONBALLOT": candidate["name_on_ballot"]
                }

                candidates.append(json.loads(render_template(candidate_template, candidate_context)))

            for area in contest["areas"]:
                area_context = {
                    "UUID": generate_uuid(),
                    "tenant_id": base_config["tenant_id"],
                    "election_event_id": election_event_id,
                    "DB_TRANS_SOURCE_ID": area["source_id"],
                    "DB_ALLMUN_AREA_NAME": area["name"],
                    "DB_POLLING_CENTER_POLLING_PLACE":area["description"]
                }

                areas.append(json.loads(render_template(area_template, area_context)))

                area_contest_context = {
                    "UUID": generate_uuid(),
                    "area_id": area_context["UUID"],
                    "contest_id": contest_context["UUID"]
                }

                area_contests.append(json.loads(render_template(area_contest_template, area_contest_context)))

    return areas, candidates, contests, area_contests, elections

# Example of how to use the function and see the result
election_tree = gen_tree(excel_data)
election_event, election_event_id = generate_election_event()

areas, candidates, contests, area_contests, elections = replace_placeholder_database(election_tree, election_event_id)

final_json = {
    "tenant_id": base_config["tenant_id"],
    "keycloak_event_realm": keycloak_template,  # Add appropriate value or leave it as is
    "election_event": election_event,  # Include the generated election event
    "elections": elections,  # Include the election objects
    "contests": contests,  # Include the contest objects
    "candidates":candidates, # Include the candidate objects
    "areas": areas,  # Include the area objects
    "area_contests": area_contests  # Include the area-contest relationships
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
