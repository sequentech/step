#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""
Script: validate_voters.py

This script reproduces the "same logic" used in your user-generation script
to verify that each row in a user CSV is correct.

**Key fix**: In order to find the correct country/embassy,
we first look up the assigned election alias for the area,
then split that alias by " - " to get an "embassy candidate" (in uppercase).
We look that up in the Keycloak dictionary (case-insensitive) to find
(official_country, official_embassy), or else fallback to EMBASSY_CANDIDATE/Unknown.

Usage:
    python validate_users.py <JSON_FILE_PATH> <CSV_FILE_PATH>
"""

import json
import sys
import csv
import os
from datetime import datetime

def main():
    # Hard-coded file paths (change if needed):
    # We need exactly two arguments: json_file_path and csv_file_path
    if len(sys.argv) < 3:
        print("USAGE: python validate_users.py <JSON_FILE_PATH> <CSV_FILE_PATH>")
        sys.exit(1)

    json_file_path = sys.argv[1]
    csv_file_path = sys.argv[2]
    error_report_file = 'error_log.txt'
    validate_clusteredPrecinct = False

    # --------------------------------------------------------------------------
    # LOAD JSON
    # --------------------------------------------------------------------------
    try:
        with open(json_file_path, 'r', encoding='utf-8') as jf:
            data = json.load(jf)
    except Exception as e:
        print(f"ERROR: Could not load JSON file '{json_file_path}': {e}")
        return

    # Parse data from JSON
    areas = data.get('areas', [])
    area_contests = data.get('area_contests', [])
    contests = data.get('contests', [])
    elections = data.get('elections', [])
    
    # Keycloak config for country/embassy
    kc_event = data.get('keycloak_event_realm', {})
    components = kc_event.get('components', {})
    uprovs = components.get('org.keycloak.userprofile.UserProfileProvider', [])
    if isinstance(uprovs, dict):
        uprovs = [uprovs]  # ensure it's a list

    # --------------------------------------------------------------------------
    # BUILD LOOKUPS
    # --------------------------------------------------------------------------
    # 1) area_name -> area_id
    area_name_to_id = {}
    for a in areas:
        a_id = a.get('id')
        a_name = a.get('name')
        if a_id and a_name:
            area_name_to_id[a_name] = a_id

    # 2) area_id -> list of contest_ids
    area_contest_map = {}
    for ac in area_contests:
        a_id = ac.get('area_id')
        c_id = ac.get('contest_id')
        area_contest_map.setdefault(a_id, []).append(c_id)

    # 3) contest_id -> election_id
    contest_election_map = {}
    for c in contests:
        c_id = c.get('id')
        e_id = c.get('election_id', 'Unknown')
        contest_election_map[c_id] = e_id

    # 4) election_id -> (alias, cluster_precinct_id)
    election_map = {}
    for el in elections:
        e_id = el.get('id')
        alias = el.get('alias', 'Unknown')
        ann = el.get('annotations', {}) or {}
        cluster_prec = ann.get('clustered_precint_id', 'Unknown')
        election_map[e_id] = (alias, cluster_prec)

    # 5) Keycloak country/embassy dictionary
    #
    #   We assume Keycloak has strings like "ROME/ITALY" or "BANGKOK/THAILAND PE".
    #   We store them in cou_emb_dict keyed by the "embassy" part in lowercase
    #   if the generation script does that. (Adjust if your generation is different!)
    #
    cou_emb_dict = {}
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
            country_attr = next((at for at in attrs if at.get('name') == 'country'), None)

            if country_attr:
                validations = country_attr.get('validations', {})
                c_opts = validations.get('options', {}).get('options', [])
                
                for opt in c_opts:
                    # e.g. "ROME/ITALY PE"
                    opt = opt.strip()
                    if '/' in opt:
                        country, embassy = opt.split('/', 1)
                        country = country.strip()
                        embassy = embassy.strip().lower()  # Store embassy keys in lowercase

                        # Ensure embassy key exists and append (country, embassy)
                        if embassy not in cou_emb_dict:
                            cou_emb_dict[embassy] = []
                        
                        cou_emb_dict[embassy].append((country, embassy.title()))  # Store embassy with proper casing

                    else:
                        # e.g. "CANADA" with no slash → country with unknown embassy
                        country = opt.strip().lower()
                        
                        if country not in cou_emb_dict:
                            cou_emb_dict[country] = []

                        cou_emb_dict[country].append((opt, "Unknown"))
    # --------------------------------------------------------------------------
    # VALIDATION
    # --------------------------------------------------------------------------
    errors = []
    
    try:
        with open(csv_file_path, 'r', newline='', encoding='utf-8') as infile:
            reader = csv.DictReader(infile)
            row_index = 1  # The header row is line 1

            for row in reader:
                row_index += 1  # Data starts on line 2

                # --------------------------------------------------------------
                # 1) area_name => must exist in JSON
                # --------------------------------------------------------------
                area_name = row.get('area_name', '').strip()
                if not area_name:
                    errors.append((row_index, 'area_name', 'Missing area_name'))
                    continue

                if area_name not in area_name_to_id:
                    errors.append((row_index, 'area_name', f"Unknown area_name '{area_name}'"))
                    continue

                a_id = area_name_to_id[area_name]
                # --------------------------------------------------------------
                # 2) Gather election aliases & clusteredPrecincts for that area
                # --------------------------------------------------------------
                contest_ids = area_contest_map.get(a_id, [])
                alias_list = []
                cluster_prec_list = []

                for cid in contest_ids:
                    e_id = contest_election_map.get(cid, 'Unknown')
                    alias, cluster_prec = election_map.get(e_id, ('Unknown', 'Unknown'))
                    alias_list.append(alias)
                    cluster_prec_list.append(cluster_prec)

                # Deduplicate aliases
                seen_alias = set()
                dedup_aliases = []
                for al in alias_list:
                    if al not in seen_alias:
                        seen_alias.add(al)
                        dedup_aliases.append(al)

                # Deduplicate clusterPrecinct
                seen_cp = set()
                dedup_cluster_precs = []
                for cp in cluster_prec_list:
                    if cp not in seen_cp:
                        seen_cp.add(cp)
                        dedup_cluster_precs.append(cp)

                # The "expected" authorized-election-ids = '|'.join(dedup_aliases)
                expected_aliases_str = '|'.join(dedup_aliases) if dedup_aliases else 'Unknown'
                row_auth_ids = row.get('authorized-election-ids', '').strip()

                if row_auth_ids != expected_aliases_str:
                    msg = (
                        f"Mismatch authorized-election-ids: got '{row_auth_ids}', "
                        f"expected '{expected_aliases_str}'"
                    )
                    errors.append((row_index, 'authorized-election-ids', msg))

                # --------------------------------------------------------------
                # 2a) NEW VALIDATION: Ensure multiple aliases share the same prefix
                #     (as previously described) ...
                # --------------------------------------------------------------
                if len(dedup_aliases) > 1:
                    # Extract the substring before ' - ' (or the whole alias if no dash)
                    prefixes = []
                    for al in dedup_aliases:
                        if ' - ' in al:
                            # everything before the dash
                            prefix = al.split(' - ', 1)[0].strip().lower()
                        else:
                            prefix = al.strip().lower()
                        prefixes.append(prefix)

                    # Check if they are all the same
                    unique_prefixes = set(prefixes)
                    if len(unique_prefixes) > 1:
                        # That means there is a mismatch among the prefixes
                        msg = (
                            "Multiple election aliases have different prefixes before ' - '. "
                            f"Found aliases={dedup_aliases}, parsed prefixes={prefixes}"
                        )
                        errors.append((row_index, 'authorized-election-ids', msg))
                # --------------------------------------------------------------
                # 2b) NEW VALIDATION for 'clusteredPrecinct'
                # --------------------------------------------------------------:
                if validate_clusteredPrecinct == True:
                    expected_cluster_prec_str = '|'.join(dedup_cluster_precs) if dedup_cluster_precs else 'Unknown'
                    row_cluster_prec = row.get('clusteredPrecinct', '').strip()

                    if row_cluster_prec != expected_cluster_prec_str:
                        msg = (
                            f"Mismatch clusteredPrecinct: got '{row_cluster_prec}', "
                            f"expected '{expected_cluster_prec_str}'"
                        )
                        errors.append((row_index, 'clusteredPrecinct', msg))
 
                # --------------------------------------------------------------
                # 3) Derive the country/embassy from the FIRST ALIAS.
                #    We do a no-case-sensitive (case-insensitive) match on the EMBASSY portion
                #    by splitting the alias on " - ", then looking up in cou_emb_dict.
                # --------------------------------------------------------------
                official_country_embassy = 'Unknown/Unknown'

                if dedup_aliases:
                    first_alias = dedup_aliases[0]

                    # Extract the EMBASSY candidate before " - " (case-insensitive)
                    if ' - ' in first_alias:
                        embassy_candidate = first_alias.split(' - ', 1)[0].strip().lower()  # Normalize for lookup

                        # Find all possible matches in `cou_emb_dict`
                        possible_matches = cou_emb_dict.get(embassy_candidate, [])

                        if isinstance(possible_matches, list) and possible_matches:
                            # Ensure all possible matches are stored in the expected format
                            official_country_embassy_list = [f"{country}/{embassy}" for country, embassy in possible_matches]
                            normalized_expected_list = [entry.lower() for entry in official_country_embassy_list]  # Normalize for comparison
                        elif isinstance(possible_matches, tuple):
                            # If a single tuple exists instead of a list
                            official_country_embassy_list = [f"{possible_matches[0]}/{possible_matches[1]}"]
                            normalized_expected_list = [official_country_embassy_list[0].lower()]
                        else:
                            # Fallback if no match is found
                            official_country_embassy_list = [f"{embassy_candidate.title()}/Unknown"]
                            normalized_expected_list = [official_country_embassy_list[0].lower()]

                    else:
                        # If no " - ", fallback to "alias/Unknown"
                        official_country_embassy_list = [f"{first_alias.title()}/Unknown"]
                        normalized_expected_list = [official_country_embassy_list[0].lower()]
                else:
                    # If no aliases are found, default to "Unknown/Unknown"
                    official_country_embassy_list = ["Unknown/Unknown"]
                    normalized_expected_list = ["unknown/unknown"]

                # Compare with the CSV row's "country" field (normalized)
                actual_country_field = row.get('country', '').strip()
                normalized_actual = actual_country_field.lower()

                if not actual_country_field:
                    errors.append((row_index, 'country', 'Missing country/embassy'))
                else:
                    if normalized_actual not in normalized_expected_list:
                        msg = (
                            f"Country mismatch: got '{actual_country_field}', "
                            f"expected one of {official_country_embassy_list} (from first alias)"
                        )
                        errors.append((row_index, 'country', msg))




                # --------------------------------------------------------------
                # 4) Validate dateOfBirth format (YYYY-MM-DD)
                # --------------------------------------------------------------
                dob = row.get('dateOfBirth', '').strip()
                if dob:
                    if not is_valid_yyyy_mm_dd(dob):
                        errors.append((row_index, 'dateOfBirth', f"Invalid date format: {dob}"))
                else:
                    errors.append((row_index, 'dateOfBirth', 'Missing dateOfBirth'))

                # (Optional) add more checks (sex in ['M','F'], etc.)

    except FileNotFoundError:
        print(f"ERROR: CSV file '{csv_file_path}' not found.")
        return
    except Exception as ex:
        print(f"ERROR reading CSV file '{csv_file_path}': {ex}")
        return

    # --------------------------------------------------------------------------
    # WRITE THE VALIDATION REPORT
    # --------------------------------------------------------------------------
    if errors:
        print(f"\nValidation completed. Found {len(errors)} issues.")
        write_error_report(error_report_file, errors)
        print(f"Validation errors saved to: {os.path.abspath(error_report_file)}")
    else:
        print("\nValidation completed. No errors found!")

def is_valid_yyyy_mm_dd(datestr):
    """Return True if datestr matches YYYY-MM-DD format."""
    try:
        datetime.strptime(datestr, "%Y-%m-%d")
        return True
    except ValueError:
        return False

def write_error_report(report_file, errors_list):
    """
    Write validation errors to a CSV or text file:
    Format: row_number, column_name, error_message
    """
    with open(report_file, 'w', newline='', encoding='utf-8') as out:
        writer = csv.writer(out)
        writer.writerow(["row_number", "column_name", "error_message"])
        for row_num, col_name, err_msg in errors_list:
            writer.writerow([row_num, col_name, err_msg])

if __name__ == "__main__":
    main()
