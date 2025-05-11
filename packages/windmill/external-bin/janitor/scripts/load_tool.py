# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env python3
import argparse
import json
import csv
import random
import re
import os
from itertools import cycle
from datetime import datetime, timedelta
import psycopg2
from psycopg2.extras import execute_values
import io
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
    election_event_file = os.path.join(working_dir, config.get("election_event_json_file", "export_election_event.json"))
    voters_config = config.get("generate_voters", {})
    areas_regex = voters_config.get("areas_regex", ".*")
    csv_file = os.path.join(working_dir, voters_config.get("csv_file_name", "generated_users.csv"))
    fields = voters_config.get("fields", [
        'username', 'last_name', 'first_name', 'middleName', 'dateOfBirth',
        'sex', 'country', 'embassy', 'clusteredPrecinct', 'overseasReferences',
        'area_name', 'authorized-election-ids', 'password', 'email', 'password_salt', 'hashed_password'
    ])
    excluded_columns = voters_config.get("excluded_columns", ['password','password_salt', 'hashed_password'])
    email_prefix = voters_config.get("email_prefix", "testsequent2025")
    domain = voters_config.get("domain", "mailinator.com")
    sequence_email_number = voters_config.get("sequence_email_number", True)
    sequence_start_number = voters_config.get("sequence_start_number", 0)
    voter_password = voters_config.get("voter_password", "Qwerty1234!")
    password_salt = voters_config.get("password_salt", "sppXH6/iePtmIgcXfTHmjPS2QpLfILVMfmmVOLPKlic=")
    hashed_password = voters_config.get("hashed_password", "V0rb8+HmTneV64qto5f0G2+OY09x2RwPeqtK605EUz0=")
    # Use Faker's date_of_birth for birth dates.
    min_age = voters_config.get("min_age", 18)
    max_age = voters_config.get("max_age", 90)
    overseas_reference = voters_config.get("overseas_reference", "B")

    # Load the JSON data
    with open(election_event_file, 'r', encoding='utf-8') as f:
        data = json.load(f)

    areas = data.get('areas', [])
    areas = [
        area
        for area in areas
        if re.match(areas_regex, area.get("name", ""))
    ]
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
    num_votes = args.num_votes
    config = load_config(working_dir)
    realm_name = config.get("realm_name", "")
    duplicate_votes_config = config.get("duplicate_votes", {})
    row_id_to_clone = duplicate_votes_config.get("row_id_to_clone", "")

    keycloak_conn = psycopg2.connect(
        dbname=os.getenv("KEYCLOAK_DB__DBNAME"),
        user=os.getenv("KEYCLOAK_DB__USER"),
        password=os.getenv("KEYCLOAK_DB__PASSWORD"),
        host=os.getenv("KEYCLOAK_DB__HOST"),
        port=os.getenv("KEYCLOAK_DB__PORT")
    )
    hasura_conn = psycopg2.connect(
        dbname=os.getenv("HASURA_DB__DBNAME"),
        user=os.getenv("HASURA_DB__USER"),
        password=os.getenv("HASURA_DB__PASSWORD"),
        host=os.getenv("HASURA_DB__HOST"),
        port=os.getenv("HASURA_DB__PORT")
    )
    print("Number of rows to clone: ", num_votes)
    kc_cursor = keycloak_conn.cursor()
    hasura_cursor = hasura_conn.cursor()
    hasura_cursor.execute(
        """
        SELECT election_id, tenant_id, area_id, annotations, content, cast_ballot_signature, election_event_id, ballot_id
            FROM sequent_backend.cast_vote WHERE id = %s""", (row_id_to_clone,))
    base_row = hasura_cursor.fetchone()
    if not base_row:
        print("No row found to clone.")
    else:
        election_id, tenant_id, area_id,annotations, content, cast_ballot_signature, election_event_id, ballot_id = base_row
        annotations_json = json.dumps(annotations)
        rows_to_insert = []

        #Offset should start at 0 and can be changed if you want to add more votes
        get_user_ids_query = """
    SELECT
        ue.id,
        ue.username,
        r.name AS realm_name
    FROM user_entity AS ue
    JOIN
        realm AS r
        ON ue.realm_id = r.id
    INNER JOIN
        user_attribute AS us
        ON us.user_id = ue.id
    WHERE
        r.name = %s AND
        us.name = 'area-id' AND
        us.value = %s
    LIMIT %s
    OFFSET 0;
        """
        print("Getting list of voters..")
        kc_cursor.execute(get_user_ids_query, (realm_name, area_id, num_votes))
        existing_user_ids = [row[0] for row in kc_cursor.fetchall()]
        print("Number of existing user ids: ", len(existing_user_ids))

        for i in range(len(existing_user_ids)):
            uid = existing_user_ids[i]
            rows_to_insert.append((
                uid, election_id, tenant_id, area_id,annotations_json, content,
                cast_ballot_signature, election_event_id, ballot_id
            ))
        print("rows_to_insert", len(rows_to_insert))
        output = io.StringIO()
        writer = csv.writer(output, delimiter='\t', quoting=csv.QUOTE_MINIMAL)
        for row in rows_to_insert:
            writer.writerow(row)
        output.seek(0)
        copy_sql = """
        COPY sequent_backend.cast_vote (
            voter_id_string, election_id, tenant_id, area_id, annotations, content,
            cast_ballot_signature, election_event_id, ballot_id
        )
        FROM STDIN WITH (FORMAT csv, DELIMITER E'\t')
        """
        start_time = datetime.now()
        print("Start time:", start_time)
        hasura_cursor.copy_expert(copy_sql, output)
        hasura_conn.commit()
        end_time = datetime.now()
        print("End time:", end_time)
    # Cleanup
    kc_cursor.close()
    keycloak_conn.close()
    hasura_cursor.close()
    hasura_conn.close()

# ------------------------------
# Action: generate-applications
# ------------------------------

def run_generate_applications(args):
    working_dir = args.working_directory
    status = args.status
    verification_type = args.type
    num_applications = args.num_applications
    config = load_config(working_dir)
    realm_name = config.get("realm_name", "")
    tenant_id = config.get("tenant_id", "")
    election_event_id = config.get("election_event_id", "")
    generate_applications_config = config.get("generate_applications", {})
    default_applicant_data = generate_applications_config.get("applicant_data", {})
    annotations = generate_applications_config.get("annotations", {})
    annotations_json = json.dumps(annotations)
    
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
    print("number of rows to clone: ", num_applications)
    kc_cursor = keycloak_conn.cursor()
    hasura_cursor = hasura_conn.cursor()
    #Offset should start at 0 and can be changed if you want to add more votes
    get_user_query = """
    SELECT 
        ue.id,
        ue.username,
        ue.email,
        ue.first_name,
        ue.last_name,
        (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'area-id' LIMIT 1) AS area_id,
        (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'country' LIMIT 1) AS country,
        (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'embassy' LIMIT 1) AS embassy,
        (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'dateOfBirth' LIMIT 1) AS dateOfBirth
    FROM user_entity ue
    JOIN realm r ON ue.realm_id = r.id
    WHERE r.name = %s
    LIMIT %s
    OFFSET 0;
    """
    kc_cursor.execute(get_user_query, (realm_name, num_applications))
    existing_users = kc_cursor.fetchall()
    print("number of existing user ids: ", len(existing_users))

    if verification_type is None:
        if status == "PENDING":
            verification_type = "MANUAL"
        else:
            verification_type = random.choice(["AUTOMATIC","MANUAL"])
    
    rows_to_insert = []
    for user in existing_users:
        user_id = user[0]
        username = user[1]
        email = user[2]
        first_name = user[3]
        last_name = user[4]
        area_id = user[5]
        country = user[6]
        embassy = user[7]
        dateOfBirth = user[8]

        # Copy the default applicant data and update with user details.
        applicant_data = default_applicant_data.copy()
        applicant_data.update({
            "email": email,
            "firstName": first_name,
            "lastName": last_name,
            "username": username,
            "country": country if country is not None else "",
            "embassy": embassy if embassy is not None else "",
            "dateOfBirth": dateOfBirth if dateOfBirth is not None else ""
        })
        applicant_data["sequent.read-only.id-card-number"] = "C" + fake.numerify("##########")

        applicant_data_json = json.dumps(applicant_data)
        rows_to_insert.append((
            user_id,
            status,
            verification_type,
            applicant_data_json,
            tenant_id,
            election_event_id,
            area_id,
            annotations_json
        ))
    print("Rows to insert:", len(rows_to_insert))
    output = io.StringIO()
    writer = csv.writer(output, delimiter='\t', quoting=csv.QUOTE_MINIMAL)
    for row in rows_to_insert:
        writer.writerow(row)
    output.seek(0)
    copy_sql = """
        COPY sequent_backend.applications (
            applicant_id, status, verification_type, applicant_data, tenant_id, election_event_id, area_id,annotations
        )
        FROM STDIN WITH (FORMAT csv, DELIMITER E'\t')
        """
    start_time = datetime.now()
    print("Start time:", start_time)
    hasura_cursor.copy_expert(copy_sql, output)
    hasura_conn.commit()
    end_time = datetime.now()
    print("End time:", end_time)

    # Cleanup
    kc_cursor.close()
    keycloak_conn.close()
    hasura_cursor.close()
    hasura_conn.close()
    
# ------------------------------
# Action: generate-activity-logs
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
    parser_voters.set_defaults(func=run_generate_voters)

    # Sub-command for duplicate-votes
    parser_votes = subparsers.add_parser("duplicate-votes", help="Duplicate cast votes in the database")
    parser_votes.add_argument("--num-votes", type=int, required=True, help="Number of votes to generate")
    parser_votes.set_defaults(func=run_duplicate_votes)


    # Sub-command for generate-applications
    parser_applications = subparsers.add_parser("generate-applications", help="Generate applications in different states")
    parser_applications.add_argument("--num-applications", type=int, required=True, help="Number of applications to generate")
    parser_applications.add_argument("--status",type=str,choices=["PENDING", "REJECTED", "ACCEPTED"], default="PENDING", help="Application status (default: PENDING)")
    parser_applications.add_argument("--type",required=False,type=str,choices=["AUTOMATIC","MANUAL"], help="Application verification type")
    parser_applications.set_defaults(func=run_generate_applications)
    
    # Sub-command for generate-activity-logs
    parser_logs = subparsers.add_parser("generate-activity-logs", help="Generate activity logs")
    parser_logs.set_defaults(func=run_generate_activity_logs)
    

    
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
