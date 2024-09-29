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

# Step 3: Parse the arguments
args = parser.parse_args()

# Step 4: Use the filename argument in your command
filename = args.filename
logging.debug(f"Filename received: {filename}")

# Step 5: Determine the script's directory to use as cwd
script_dir = os.path.dirname(os.path.abspath(__file__))

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

# Step 13: Prepare context for rendering
context = {
#    "UUID": generate_uuid(),
    "current_timestamp": current_timestamp,
    "tenant_id": base_config["tenant_id"],
    "miru_election-event-id": base_config["election_event"]["miru_election-event-id"],
    "miru_election-id": base_config["election"]["miru_election-id"],
    "miru_election-event-name": base_config["election_event"]["miru_election-event-name"],
    "miru_election-name": base_config["election"]["miru_election-name"],
    "election_event_name": base_config["election_event"]["name"],
    "election_name": base_config["election"]["name"],
    "election_event_description": base_config["election_event"]["description"],
    "election_event_logo_url": base_config["election_event"]["logo_url"],
    # Add other replacements as needed from SQLite queries
}

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

def gen_tree():
    results = get_data()
    elections_object = {"elections": []}

    for row in results:
        # Find or create the election object
        election = next((e for e in elections_object["elections"] if e["election_post"] == row["DB_POLLING_CENTER_POLLING_PLACE"]), None)
        
        if not election:
            # If the election does not exist, create it
            election = {
                "election_post": row["DB_POLLING_CENTER_POLLING_PLACE"],
                "election_name":context["election_name"],
                "contests": [],
                **context
            }
            elections_object["elections"].append(election)

        # Find or create the contest object within the election
        contest = next((c for c in election["contests"] if c["name"] == row["DB_CONTEST_NAME"]), None)
        
        if not contest:
            # If the contest does not exist, create it
            contest = {
                "name": row["DB_CONTEST_NAME"],
                "eligible_amount": row["DB_RACE_ELIGIBLEAMOUNT"],
                "district_code": row["DB_SEAT_DISTRICTCODE"],
                "postcode": row["contest_POSTCODE"],
                "sort_order": row["contest_SORT_ORDER"],
                "candidates": [],
                "areas": [],
                **context
            }
            election["contests"].append(contest)

        # Add the candidate to the contest
        candidate = {
            "code": row["DB_CANDIDATE_CAN_CODE"],
            "name_on_ballot": row["DB_CANDIDATE_NAMEONBALLOT"],
            "nominated_by": row["DB_CANDIDATE_NOMINATEDBY"],
            "party_short_name": row["DB_PARTY_SHORT_NAME"],
            "party_name": row["DB_PARTY_NAME_PARTY"],
            **context
        }
        contest["candidates"].append(candidate)

        # Add the area to the contest if it hasn't been added already
        area = {
            "name": row["DB_ALLMUN_AREA_NAME"],
            "description" :row["DB_POLLING_CENTER_POLLING_PLACE"],
            "source_id": row["DB_TRANS_SOURCE_ID"],
            "dest_id": row["trans_route_TRANS_DEST_ID"],
            **context
        }
        
        if area not in contest["areas"]:
            contest["areas"].append(area)

    return elections_object


def remplace_placeholder_database(election_tree, election_event_id):
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
election_tree = gen_tree()
election_event, election_event_id = generate_election_event()

areas, candidates, contests, area_contests, elections = remplace_placeholder_database(election_tree, election_event_id)

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
