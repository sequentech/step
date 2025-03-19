#!/usr/bin/env python3
"""
Script: generate_users_from_json.py

This script:
1) Prompts the user for how many users to generate.
2) Reads the JSON file (the election event export).
3) Cycles through the available areas in the JSON to produce user records.
4) Generates a CSV with user data:
   username, last_name, first_name, middleName, dateOfBirth, sex,
   country, embassy, clusteredPrecinct, overseasReferences, area_name,
   authorized-election-ids, password.

Key logic:
----------
- We parse 'org.keycloak.userprofile.UserProfileProvider' (a list) -> first item.
  Then 'config' -> 'kc.user.profile.config' (list) -> parse its first JSON.
  Inside that, we find the attribute named 'country' with an array of "Country/Embassy"
  that we store in a dict for easy matching.
- For each area:
  - We parse 'area_name'. If it has ' - ', we take the text before the dash as the 'country candidate'.
  - We do a case-insensitive lookup in our dict. If found, we fill official country & embassy.
    Otherwise, fallback.

  - Then from 'area_contests', we see which 'contest_id's are assigned to this area.
  - For each assigned contest, we look up 'contest.election_id' from the 'contests'.
    Then we find that election in 'elections' to retrieve:
    * election.alias => goes in 'authorized-election-ids'
    * election.annotations.clustered_precint_id => goes in 'clusteredPrecinct'
  - We gather them into lists, remove duplicates while preserving order, and join with '|'.

- dateOfBirth is random (1970..2022). last_name, first_name, sex are random from small lists.
- overseasReferences is 'B', password is 'Qwerty1234!'
- "email" that starts with: {email_prefix}+{random_number}@{domain}
Usage:
------
1) Place this script & JSON in the same folder.
2) Update 'json_file_path' if needed.
3) 'python generate_users_from_json.py', specify how many users.
4) 'generated_users.csv' is the output.
"""

import json
import csv
import random
import os
from itertools import cycle
from datetime import datetime, timedelta

# Adjust if needed
json_file_path = './export_election_event-fdaa8658-455b-4b73-93e2-a2fac8667b3c.json'
csv_file_path = 'generated_users_1000.csv'
fields = [
    'username', 'last_name', 'first_name', 'middleName', 'dateOfBirth',
    'sex', 'country', 'embassy', 'clusteredPrecinct', 'overseasReferences',
    'area_name', 'authorized-election-ids', 'password','email', 'password_salt','hashed_password'
]
# Decide which columns to exclude for now.
excluded_columns = ['password','password_salt', 'hashed_password']

# Configure user email
email_prefix = 'testsequent2025'
domain = 'mailinator.com'
sequence_email_number = True # This make the email sequence of random
sequence_start_number = 0

# Set voter passwords
password = 'Qwerty1234!'

# Confguration For load test
password_salt ='sppXH6/iePtmIgcXfTHmjPS2QpLfILVMfmmVOLPKlic='
hashed_password ='V0rb8+HmTneV64qto5f0G2+OY09x2RwPeqtK605EUz0='

########################################
# Generate a random date of birth (YYYY-mm-dd)
########################################
def random_date(start_year=1970, end_year=2022):
    start_date = datetime(start_year, 1, 1)
    end_date = datetime(end_year, 12, 31)
    delta = end_date - start_date
    random_days = random.randint(0, delta.days)
    result_date = start_date + timedelta(days=random_days)
    return result_date.strftime('%Y-%m-%d')

########################################
# Helper: remove duplicates while preserving order
########################################
def deduplicate_preserve_order(items):
    seen = set()
    result = []
    for it in items:
        if it not in seen:
            seen.add(it)
            result.append(it)
    return result

########################################
# Load the JSON data
########################################
with open(json_file_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

areas = data.get('areas', [])
area_contests = data.get('area_contests', [])
contests = data.get('contests', [])
elections = data.get('elections', [])  # needed to find election alias & clusterPrecint

########################################
# Build a map: election.id -> (alias, cluster_precinct_id)
########################################
election_map = {}  # e_id -> (alias, clusterPrec)
for el in elections:
    e_id = el.get('id')
    alias = el.get('alias', 'Unknown')
    ann = el.get('annotations', {})
    # clusterPrecint from election.annotations
    cluster_prec = ann.get('clustered_precint_id', 'Unknown')
    election_map[e_id] = (alias, cluster_prec)

########################################
# area_id -> list of contest_ids
########################################
area_contest_map = {}
for ac in area_contests:
    a_id = ac.get('area_id')
    c_id = ac.get('contest_id')
    if a_id not in area_contest_map:
        area_contest_map[a_id] = []
    area_contest_map[a_id].append(c_id)

########################################
# For each contest: contest.election_id => find in elections
########################################
contest_election_map = {}  # c_id -> election_id
for c in contests:
    c_id = c.get('id')
    e_id = c.get('election_id', 'Unknown')
    contest_election_map[c_id] = e_id

########################################
# Parse Keycloak config for country/embassy
########################################
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
                # e.g. 'Jordan/Amman PE'
                if '/' in opt:
                    ctry, emb = opt.split('/', 1)
                    ctry = ctry.strip()
                    emb = emb.strip()
                    cou_emb_dict[emb.lower()] = (ctry, emb)
                else:
                    cou_emb_dict[opt.lower()] = (opt.strip(), 'Unknown')

########################################
# Some name lists for random picking
########################################
first_names = [
    "Michael", "Christopher", "Jessica", "Matthew", "Ashley", "Jennifer", "Joshua", "Amanda", "Daniel", "David", "James", "Robert", "John", "Joseph", "Andrew", "Ryan", "Brandon", "Jason", "Justin", "Sarah", "William", "Jonathan", "Stephanie", "Brian", "Nicole", "Nicholas", "Anthony", "Heather", "Eric", "Elizabeth", "Adam", "Megan", "Melissa", "Kevin", "Steven", "Thomas", "Timothy", "Christina", "Kyle", "Rachel", "Laura", "Lauren", "Amber", "Brittany", "Danielle", "Richard", "Kimberly", "Jeffrey", "Amy", "Crystal", "Michelle", "Tiffany", "Jeremy", "Benjamin", "Mark", "Emily", "Jacob", "Stephen", "Patrick", "Sean", "Erin", "Zachary", "Jamie", "Kelly", "Samantha", "Nathan", "Sara", "Dustin", "Paul", "Angela", "Tyler", "Nicole", "Andrea", "Kristen", "Craig", "Erica", "Vanessa", "Travis", "Lisa", "Devin", "Erika", "Jared", "Sabrina", "Jordan", "Alexander", "Seth", "Brianna", "Oliver", "Charlotte", "Emma", "Harper", "Ava", "Isabella", "Evelyn", "Sophia", "Amelia", "Mia", "Abigail", "Liam", "Noah" 
]
last_names = [
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez", "Thompson", "White", "Harris", "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson", "Walker", "Young", "Allen", "King", "Wright", "Scott", "Torres", "Nguyen", "Hill", "Flores", "Green", "Adams", "Nelson", "Baker", "Hall", "Rivera", "Campbell", "Mitchell", "Carter", "Roberts", "Gomez", "Phillips", "Evans", "Turner", "Diaz", "Parker", "Cruz", "Edwards", "Collins", "Reyes", "Stewart", "Morris", "Morales", "Murphy", "Cook", "Rogers", "Gutierrez", "Ortiz", "Morgan", "Cooper", "Peterson", "Bailey", "Reed", "Kelly", "Howard", "Ramos", "Kim", "Cox", "Ward", "Richardson", "Watson", "Brooks", "Chavez", "Wood", "James", "Bennett", "Gray", "Mendoza", "Ruiz", "Hughes", "Price", "Alvarez", "Castillo", "Sanders", "Patel", "Myers", "Long", "Ross", "Foster", "Jimenez", "Powell", "Jenkins"
]

########################################
# Prompt
########################################
num_users = int(input('Enter the number of users to generate: '))
area_cycle = cycle(areas)

users = []
username_counter = 1000001

########################################
# Generate
########################################
for i in range(num_users):
    area = next(area_cycle)
    area_id = area.get('id')
    area_name = area.get('name', 'Unknown')

    # For this area, gather assigned contests
    assigned_cids = area_contest_map.get(area_id, [])

    # Build lists of election aliases & precincts from the election_map
    election_aliases = []
    precincts = []

    for cid in assigned_cids:
        e_id = contest_election_map.get(cid, 'Unknown')
        alias, cluster_prec = election_map.get(e_id, ('Unknown', 'Unknown'))
        election_aliases.append(alias)
        precincts.append(cluster_prec)

    # remove duplicates while preserving order
    election_aliases = deduplicate_preserve_order(election_aliases)
    precincts = deduplicate_preserve_order(precincts)

    if ' - ' in election_aliases[0]:
        election_country_candidate = election_aliases[0].split(' - ', 1)[0].strip()
    else:
        election_country_candidate = election_aliases[0].strip()

    lookup_key = election_country_candidate.lower()
    if lookup_key in cou_emb_dict:
        official_country, official_embassy = cou_emb_dict[lookup_key]
    else:
        official_country = election_country_candidate
        official_embassy = 'Unknown'

    joined_aliases = '|'.join(election_aliases) if election_aliases else 'Unknown'
    joined_precincts = '|'.join(precincts) if precincts else 'Unknown'

    dob = random_date()

    if sequence_email_number == True:
        # Generate email: {email_prefix}{sequence_number}@{domain}
        email = f"{email_prefix}+{i + 10000001 +sequence_start_number}@{domain}"
    else:
        # Generate email: {email_prefix}{random_number}@{domain}
        random_num = random.randint(100000, 999999999)
        email = f"{email_prefix}+{random_num}@{domain}"

    
        

    user_record = {
        'username': username_counter,
        'last_name': random.choice(last_names),
        'first_name': random.choice(first_names),
        'middleName': '',
        'dateOfBirth': dob,
        'sex': random.choice(['M', 'F']),
        'country': official_country+'/'+official_embassy,
        'embassy': official_embassy,
        'clusteredPrecinct': joined_precincts,
        'overseasReferences': 'B',
        'area_name': 'ISRAEL - TEL AVIV PE',
        'authorized-election-ids': joined_aliases,
        'password': password,
        'email':email,
        'password_salt':password_salt,
        'hashed_password': hashed_password
    }

    users.append(user_record)
    username_counter += 1

########################################
# Write CSV
########################################

# Filter out the excluded columns
final_fields = [f for f in fields if f not in excluded_columns]

with open(csv_file_path, 'w', newline='', encoding='utf-8') as outfile:
    writer = csv.DictWriter(outfile, fieldnames=final_fields)
    writer.writeheader()
    for u in users:
        outrow = u.copy()
        outrow["username"] = str(outrow["username"])
        # Remove keys not in final_fields
        for k in list(outrow.keys()):
            if k not in final_fields:
                del outrow[k]
        writer.writerow(outrow)

print(f"Successfully generated {num_users} users. CSV file created at: {os.path.abspath(csv_file_path)}")