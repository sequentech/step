# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env python3
import argparse
import json
import csv
import random
import os
from itertools import cycle
from datetime import datetime, timedelta
import psycopg2
from psycopg2.extras import execute_values
from faker import Faker

# Initialize Faker
fake = Faker()

# ------------------------------
# Utility Functions
# ------------------------------

def load_config(working_dir):
    config_path = os.path.join(working_dir, "config.json")
    with open(config_path, "r", encoding="utf-8") as f:
        return json.load(f)

def deduplicate_preserve_order(items):
    seen = set()
    result = []
    for it in items:
        if it not in seen:
            seen.add(it)
            result.append(it)
    return result

# ------------------------------
# Action: generate-voters
# ------------------------------

def run_generate_voters(args):
    working_dir = args.working_directory
    num_users = args.num_users
    config = load_config(working_dir)
    
    # Load settings from config.json
    json_file = os.path.join(working_dir, config.get("json_file", "export_election_event.json"))
    csv_file = os.path.join(working_dir, config.get("csv_file", "generated_users.csv"))
    fields = config.get("fields", [
        'username', 'last_name', 'first_name', 'middleName', 'dateOfBirth',
        'sex', 'country', 'embassy', 'clusteredPrecinct', 'overseasReferences',
        'area_name', 'authorized-election-ids', 'password', 'email', 'password_salt', 'hashed_password'
    ])
    excluded_columns = config.get("excluded_columns", ['password','password_salt', 'hashed_password'])
    email_prefix = config.get("email_prefix", "testsequent2025")
    domain = config.get("domain", "mailinator.com")
    sequence_email_number = config.get("sequence_email_number", True)
    sequence_start_number = config.get("sequence_start_number", 0)
    voter_password = config.get("voter_password", "Qwerty1234!")
    password_salt = config.get("password_salt", "sppXH6/iePtmIgcXfTHmjPS2QpLfILVMfmmVOLPKlic=")
    hashed_password = config.get("hashed_password", "V0rb8+HmTneV64qto5f0G2+OY09x2RwPeqtK605EUz0=")
    # Use Faker's date_of_birth for birth dates.
    min_age = config.get("min_age", 18)
    max_age = config.get("max_age", 90)
    overseas_reference = config.get("overseas_reference", "B")

    # Load the JSON data
    with open(json_file, 'r', encoding='utf-8') as f:
        data = json.load(f)

    areas = data.get('areas', [])
    area_contests = data.get('area_contests', [])
    contests = data.get('contests', [])
    elections = data.get('elections', [])

    # Build election map: election.id -> (alias, clustered precinct)
    election_map = {}
    for el in elections:
        e_id = el.get('id')
        alias = el.get('alias', 'Unknown')
        ann = el.get('annotations', {})
        cluster_prec = ann.get('clustered_precint_id', 'Unknown')
        election_map[e_id] = (alias, cluster_prec)

    # Build area -> contest mapping
    area_contest_map = {}
    for ac in area_contests:
        a_id = ac.get('area_id')
        c_id = ac.get('contest_id')
        if a_id not in area_contest_map:
            area_contest_map[a_id] = []
        area_contest_map[a_id].append(c_id)

    # Build contest to election mapping
    contest_election_map = {}
    for c in contests:
        c_id = c.get('id')
        e_id = c.get('election_id', 'Unknown')
        contest_election_map[c_id] = e_id

    # Parse Keycloak config for country/embassy
    cou_emb_dict = {}
    kc_event = data.get('keycloak_event_realm', {})
    components = kc_event.get('components', {})
    uprovs = components.get('org.keycloak.userprofile.UserProfileProvider', [])
    if isinstance(uprovs, dict):
        uprovs = [uprovs]
    if uprovs:
        first_uprov = uprovs[0]
        conf = first_uprov.get('config', {})
        kc_conf_list = conf.get('kc.user.profile.config', [])
        if kc_conf_list:
            raw_json_str = kc_conf_list[0]
            try:
                user_profile_config = json.loads(raw_json_str)
            except Exception:
                user_profile_config = {}
            attrs = user_profile_config.get('attributes', [])
            country_attr = None
            for at in attrs:
                if at.get('name') == 'country':
                    country_attr = at
                    break
            if country_attr:
                validations = country_attr.get('validations', {})
                c_opts = validations.get('options', {}).get('options', [])
                for opt in c_opts:
                    if '/' in opt:
                        ctry, emb = opt.split('/', 1)
                        cou_emb_dict[emb.lower()] = (ctry.strip(), emb.strip())
                    else:
                        cou_emb_dict[opt.lower()] = (opt.strip(), 'Unknown')

    users = []
    username_counter = 1
    area_cycle = cycle(areas)

    for i in range(num_users):
        area = next(area_cycle)
        area_id = area.get('id')
        area_name = area.get('name', 'Unknown')
        assigned_cids = area_contest_map.get(area_id, [])
        election_aliases = []
        precincts = []
        for cid in assigned_cids:
            e_id = contest_election_map.get(cid, 'Unknown')
            alias, cluster_prec = election_map.get(e_id, ('Unknown', 'Unknown'))
            election_aliases.append(alias)
            precincts.append(cluster_prec)
        election_aliases = deduplicate_preserve_order(election_aliases)
        precincts = deduplicate_preserve_order(precincts)
        if election_aliases and ' - ' in election_aliases[0]:
            election_country_candidate = election_aliases[0].split(' - ', 1)[0].strip()
        elif election_aliases:
            election_country_candidate = election_aliases[0].strip()
        else:
            election_country_candidate = 'Unknown'
        lookup_key = election_country_candidate.lower()
        if lookup_key in cou_emb_dict:
            official_country, official_embassy = cou_emb_dict[lookup_key]
        else:
            official_country = election_country_candidate
            official_embassy = 'Unknown'
        joined_aliases = '|'.join(election_aliases) if election_aliases else 'Unknown'
        joined_precincts = '|'.join(precincts) if precincts else 'Unknown'
        # Generate date of birth using Faker's built-in method.
        dob = fake.date_of_birth(minimum_age=min_age, maximum_age=max_age).strftime('%Y-%m-%d')

        if sequence_email_number:
            email = f"{email_prefix}+{i+sequence_start_number}@{domain}"
        else:
            random_num = random.randint(100000, 999999999)
            email = f"{email_prefix}+{random_num}@{domain}"

        user_record = {
            'username': username_counter,
            'first_name': fake.first_name(),
            'last_name': fake.last_name(),
            'middleName': '',
            'dateOfBirth': dob,
            'sex': random.choice(['M', 'F']),
            'country': f"{official_country}/{official_embassy}",
            'embassy': official_embassy,
            'clusteredPrecinct': joined_precincts,
            'overseasReferences': overseas_reference,
            'area_name': area_name,
            'authorized-election-ids': joined_aliases,
            'password': voter_password,
            'email': email,
            'password_salt': password_salt,
            'hashed_password': hashed_password
        }
        users.append(user_record)
        username_counter += 1

    final_fields = [f for f in fields if f not in excluded_columns]
    with open(csv_file, 'w', newline='', encoding='utf-8') as outfile:
        writer = csv.DictWriter(outfile, fieldnames=final_fields)
        writer.writeheader()
        for u in users:
            outrow = u.copy()
            outrow["username"] = str(outrow["username"])
            for k in list(outrow.keys()):
                if k not in final_fields:
                    del outrow[k]
            writer.writerow(outrow)

    print(f"Successfully generated {num_users} users. CSV file created at: {os.path.abspath(csv_file)}")

# ------------------------------
# Action: duplicate-votes
# ------------------------------

def run_duplicate_votes(args):
    working_dir = args.working_directory
    config = load_config(working_dir)
    duplicate_votes_config = config.get("duplicate_votes", {})
    realm_name = duplicate_votes_config.get("realm_name", "")
    target_row_count = duplicate_votes_config.get("target_row_count", 100)
    row_id_to_clone = duplicate_votes_config.get("row_id_to_clone", "")

    # Connect using environment variables
    keycloak_conn = psycopg2.connect(
        dbname=os.getenv("KC_DB"),
        user=os.getenv("KC_DB_USERNAME"),
        password=os.getenv("KC_DB_PASSWORD"),
        host=os.getenv("KC_DB_URL_HOST"),
        port=os.getenv("KC_DB_URL_PORT")
    )
    hasura_conn = psycopg2.connect(
        dbname=os.getenv("HASURA_PG_DBNAME"),
        user=os.getenv("HASURA_PG_USER"),
        password=os.getenv("HASURA_PG_PASSWORD"),
        host=os.getenv("HASURA_PG_HOST"),
        port=os.getenv("HASURA_PG_PORT")
    )

    print("Number of rows to clone:", target_row_count)

    kc_cursor = keycloak_conn.cursor()
    hasura_cursor = hasura_conn.cursor()

    get_user_ids_query = """
    SELECT ue.id
    FROM user_entity AS ue
    JOIN realm AS r ON ue.realm_id = r.id
    WHERE r.name = %s
    LIMIT %s;
    """
    kc_cursor.execute(get_user_ids_query, (realm_name, target_row_count))
    existing_user_ids = [row[0] for row in kc_cursor.fetchall()]
    print("Number of existing user ids:", len(existing_user_ids))

    hasura_cursor.execute(
        """
        SELECT election_id, tenant_id, area_id, content, cast_ballot_signature, election_event_id, ballot_id
        FROM sequent_backend.cast_vote WHERE id = %s
        """, (row_id_to_clone,))
    base_row = hasura_cursor.fetchone()

    if not base_row:
        print("No row found to clone.")
    else:
        election_id, tenant_id, area_id, content, cast_ballot_signature, election_event_id, ballot_id = base_row
        insert_query = """
        INSERT INTO sequent_backend.cast_vote (
            voter_id_string, election_id, tenant_id, area_id, content,
            cast_ballot_signature, election_event_id, ballot_id
        )
        VALUES %s
        """
        rows_to_insert = []
        for i, uid in enumerate(existing_user_ids):
            rows_to_insert.append((
                uid, election_id, tenant_id, area_id, content,
                cast_ballot_signature, election_event_id, ballot_id
            ))
            print("Preparing row", i)
        
        execute_values(hasura_cursor, insert_query, rows_to_insert, page_size=1000)
        hasura_conn.commit()
        print("Duplicate votes inserted successfully.")

    kc_cursor.close()
    keycloak_conn.close()
    hasura_cursor.close()
    hasura_conn.close()

# ------------------------------
# Action: generate-applications (Stub)
# ------------------------------

def run_generate_applications(args):
    print("generate-applications action is not implemented yet.")
    
# ------------------------------
# Action: generate-activity-logs (Stub)
# ------------------------------

def run_generate_activity_logs(args):
    print("generate-activity-logs action is not implemented yet.")

# ------------------------------
# Main Dispatcher
# ------------------------------

def main():
    parser = argparse.ArgumentParser(description="Load Testing Tool")
    subparsers = parser.add_subparsers(dest="action", required=True, help="Action to perform")
    
    # Global argument for working directory
    parser.add_argument("--working-directory", required=True, help="Path to working directory (input/output directory)")

    # Sub-command for generate-voters
    parser_voters = subparsers.add_parser("generate-voters", help="Generate random voters CSV file")
    parser_voters.add_argument("--num-users", type=int, required=True, help="Number of users to generate")
    
    # Sub-command for duplicate-votes
    subparsers.add_parser("duplicate-votes", help="Duplicate cast votes in the database")
    
    # Sub-command for generate-applications
    subparsers.add_parser("generate-applications", help="Generate applications in different states")
    
    # Sub-command for generate-activity-logs
    subparsers.add_parser("generate-activity-logs", help="Generate activity logs")
    

    
    args = parser.parse_args()
    
    if args.action == "generate-voters":
        run_generate_voters(args)
    elif args.action == "duplicate-votes":
        run_duplicate_votes(args)
    elif args.action == "generate-applications":
        run_generate_applications(args)
    elif args.action == "generate-activity-logs":
        run_generate_activity_logs(args)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
