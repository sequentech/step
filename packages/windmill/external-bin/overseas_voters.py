#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

import os
import sys
import csv
import argparse
import psycopg2
from psycopg2.extras import RealDictCursor
import logging
from typing import List, Dict, Optional, Tuple
import tempfile
import subprocess # For system sort
import shutil # For fallback copy operations
import re # For filename sanitization

# --- Global Constants ---
REALM_NAME = "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-e3713d09-a955-48ad-9bc4-4177a158a3f1"
TENANT_ID = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
ELECTION_EVENT_ID_CONST = "e3713d09-a955-48ad-9bc4-4177a158a3f1"

AREA_ID_ATTR_NAME = "area-id"
VALIDATE_ID_ATTR_NAME = "validate_id"
VALIDATE_ID_REGISTERED_VOTER = "registered-voter"

FINAL_CSV_FIELDNAMES = [
    'Voter ID', 'Username', 'First Name', 'Middle Name', 'Last Name',
    'Suffix', 'Area', 'Registered', 'Voted', 'Vote Date'
]
SYSTEM_SORT_PRIMARY_KEYS = ['Last Name', 'First Name', 'Middle Name']

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

HASURA_DB_CONFIG = {
    'user': os.environ.get('HASURA_DB__USER'),
    'password': os.environ.get('HASURA_DB__PASSWORD'),
    'host': os.environ.get('HASURA_DB__HOST'),
    'port': os.environ.get('HASURA_DB__PORT', '5432'),
    'dbname': os.environ.get('HASURA_DB__DBNAME', 'hasura')
}
KEYCLOAK_DB_CONFIG = {
    'user': os.environ.get('KEYCLOAK_DB__USER'),
    'password': os.environ.get('KEYCLOAK_DB__PASSWORD'),
    'host': os.environ.get('KEYCLOAK_DB__HOST'),
    'port': os.environ.get('KEYCLOAK_DB__PORT', '5432'),
    'dbname': os.environ.get('KEYCLOAK_DB__DBNAME', 'keycloak')
}

def connect_to_db(config: Dict[str, str]):
    try:
        conn = psycopg2.connect(**config)
        return conn
    except Exception as e:
        logger.error(f"Database connection error: {e}")
        raise

def sanitize_filename_component(name: str, max_length: int = 20) -> str:
    if not name:
        return "unnamed"
    name = str(name).lower()
    name = re.sub(r'\s+', '_', name)
    name = re.sub(r'[^\w\-_.]', '', name)
    name = name[:max_length]
    name = name.rstrip('-_.')
    return name or "sanitized_name"

def get_all_non_test_elections(hasura_conn, tenant_id: str, election_event_id: str) -> List[Dict]:
    logger.info(f"Fetching all non-test elections for tenant_id={tenant_id} and election_event_id={election_event_id}")
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute(f"""
            SELECT id, alias, name
            FROM sequent_backend.election
            WHERE tenant_id = '{tenant_id}' AND election_event_id = '{election_event_id}'
            AND name NOT ILIKE '%Test%' AND (alias IS NULL OR alias NOT ILIKE '%Test%')
            ORDER BY alias, name, id
        """)
        elections = cursor.fetchall()
        logger.info(f"Found {len(elections)} non-test elections.")
        return elections

def get_areas_by_election_id(hasura_conn, tenant_id: str, election_event_id: str, election_id: str) -> List[Dict]:
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute(f"""
            SELECT DISTINCT ON (a.id) a.* FROM sequent_backend.area a
            JOIN sequent_backend.area_contest ac ON a.id = ac.area_id AND a.election_event_id = ac.election_event_id AND a.tenant_id = ac.tenant_id
            JOIN sequent_backend.contest c ON ac.contest_id = c.id AND ac.election_event_id = c.election_event_id AND ac.tenant_id = c.tenant_id
            WHERE c.tenant_id = '{tenant_id}' AND c.election_event_id = '{election_event_id}' AND c.election_id = '{election_id}'
            ORDER BY a.id
        """, ())
        return cursor.fetchall()

def dump_all_voters_to_file_copy(keycloak_conn, realm_name: str, area_id_value: str, output_filepath: str) -> int:
    logger.debug(f"Dumping voters for area {area_id_value} to {output_filepath} using COPY")
    sql_query = f"""
    SELECT
        u.id,
        COALESCE(u.username, '') AS username,
        COALESCE(u.first_name, '') AS first_name,
        COALESCE(attr_json.attributes ->> 'middleName', '') AS middle_name,
        COALESCE(u.last_name, '') AS last_name,
        COALESCE(attr_json.attributes ->> 'suffix', '') AS suffix,
        CASE
            WHEN COALESCE(attr_json.attributes ->> '{VALIDATE_ID_ATTR_NAME}', '') = '{VALIDATE_ID_REGISTERED_VOTER}' THEN 'Yes'
            ELSE 'No'
        END AS registered_str
    FROM user_entity u
    INNER JOIN realm AS ra ON ra.id = u.realm_id
    LEFT JOIN LATERAL (
        SELECT json_object_agg(ua.name, ua.value) AS attributes
        FROM user_attribute ua WHERE ua.user_id = u.id GROUP BY ua.user_id
    ) attr_json ON true
    WHERE
        ra.name = %s AND
        EXISTS (
            SELECT 1 FROM user_attribute ua_area
            WHERE ua_area.user_id = u.id
            AND ua_area.name = '{AREA_ID_ATTR_NAME}'
            AND ua_area.value = %s
        )
    ORDER BY u.id
    """
    rows_dumped = 0
    try:
        with open(output_filepath, 'wb') as f_voters:
            with keycloak_conn.cursor() as cursor:
                sub_query_for_copy = cursor.mogrify(sql_query, (realm_name, area_id_value)).decode(keycloak_conn.encoding)
                final_copy_command = f"COPY ({sub_query_for_copy}) TO STDOUT WITH (FORMAT CSV, HEADER TRUE)"
                cursor.copy_expert(final_copy_command, f_voters)
                rows_dumped = cursor.rowcount
        logger.debug(f"Finished dumping {rows_dumped} voters for area {area_id_value} using COPY.")
    except Exception as e:
        logger.error(f"Error dumping voters for area {area_id_value} using COPY: {e}", exc_info=True)
        raise
    return rows_dumped

def dump_all_cast_votes_to_file_copy(
    hasura_conn, tenant_id: str, election_event_id: str, election_id: str, area_id_value: str, output_filepath: str
) -> int:
    logger.debug(f"Dumping cast votes for area {area_id_value} to {output_filepath} using COPY")
    sql_query = f"""
    WITH RankedVotes AS (
        SELECT
            v.voter_id_string,
            v.created_at AS voted_date,
            ROW_NUMBER() OVER(PARTITION BY v.voter_id_string ORDER BY v.created_at DESC) as rn
        FROM sequent_backend.cast_vote v
        JOIN sequent_backend.election e ON
            v.election_id = e.id
            AND v.tenant_id = e.tenant_id
            AND v.election_event_id = e.election_event_id
        WHERE
            v.tenant_id = '{tenant_id}'
            AND v.election_event_id = '{election_event_id}'
            AND v.election_id = '{election_id}'
            AND v.area_id = '{area_id_value}'
    )
    SELECT voter_id_string AS voter_id, to_char(voted_date, 'YYYY-MM-DD"T"HH24:MI:SS.USOF') AS voted_date_iso
    FROM RankedVotes WHERE rn = 1 ORDER BY voter_id_string
    """
    rows_dumped = 0
    try:
        with open(output_filepath, 'wb') as f_votes:
            with hasura_conn.cursor() as cursor:
                sub_query_for_copy = cursor.mogrify(sql_query).decode(hasura_conn.encoding)
                final_copy_command = f"COPY ({sub_query_for_copy}) TO STDOUT WITH (FORMAT CSV, HEADER TRUE)"
                cursor.copy_expert(final_copy_command, f_votes)
                rows_dumped = cursor.rowcount
        logger.debug(f"Finished dumping {rows_dumped} cast votes for area {area_id_value} using COPY.")
    except Exception as e:
        logger.error(f"Error dumping cast votes for area {area_id_value} using COPY: {e}", exc_info=True)
        raise
    return rows_dumped

def process_area(
    area_dict: Dict, tenant_id: str, election_event_id: str, keycloak_realm_name: str,
    election_id: str, output_dir: str, # output_dir is now election_specific_output_dir
    hasura_conn, keycloak_conn
) -> Tuple[Optional[str], int, int, int]:
    area_id = area_dict['id']
    area_name = area_dict.get('name', 'UnknownArea')
    area_name_cleaned = sanitize_filename_component(area_name, 20)
    
    # Simplified area report filename, as it's inside an election-specific folder
    area_report_filename = os.path.join(output_dir, f"area_{area_name_cleaned}_report.csv")

    logger.info(f"Processing area: {area_name} (ID: {area_id}). Output file: {area_report_filename}")

    temp_voters_file, temp_votes_file = None, None
    num_voters_for_area, num_votes_for_area, written_records_count = 0, 0, 0

    try:
        with tempfile.NamedTemporaryFile(mode='wb', delete=False, prefix=f"area_{area_id}_voters_raw_", suffix='.csv') as tmp_f_v:
            temp_voters_file = tmp_f_v.name
        with tempfile.NamedTemporaryFile(mode='wb', delete=False, prefix=f"area_{area_id}_votes_raw_", suffix='.csv') as tmp_f_c:
            temp_votes_file = tmp_f_c.name

        num_voters_for_area = dump_all_voters_to_file_copy(keycloak_conn, keycloak_realm_name, area_id, temp_voters_file)
        num_votes_for_area = dump_all_cast_votes_to_file_copy(hasura_conn, tenant_id, election_event_id, election_id, area_id, temp_votes_file)
        
        logger.info(f"Area {area_name}: Dumped {num_voters_for_area} voters, {num_votes_for_area} cast votes (raw).")

        voter_copy_fieldnames = ['id', 'username', 'first_name', 'middle_name', 'last_name', 'suffix', 'registered_str']
        vote_copy_fieldnames = ['voter_id', 'voted_date_iso']

        if num_voters_for_area == 0: # If no voters, report will be header only for this area.
             logger.info(f"No voters found for area {area_name}. Creating empty report (header only).")
        
        with open(area_report_filename, 'w', newline='', encoding='utf-8') as area_csv_f:
            main_writer = csv.DictWriter(area_csv_f, fieldnames=FINAL_CSV_FIELDNAMES)
            main_writer.writeheader()

            if num_voters_for_area > 0 : # Proceed with merge only if there are voters
                with open(temp_voters_file, 'r', newline='', encoding='utf-8') as voters_f, \
                     open(temp_votes_file, 'r', newline='', encoding='utf-8') as votes_f:
                    
                    voter_reader = csv.DictReader(voters_f) # Header is from COPY
                    vote_reader = csv.DictReader(votes_f)   # Header is from COPY
                    
                    current_voter = next(voter_reader, None)
                    current_vote = next(vote_reader, None) if num_votes_for_area > 0 else None

                    while current_voter:
                        v_id = current_voter['id']
                        voted, vote_date = 'No', ''
                        
                        while current_vote and current_vote.get('voter_id') and current_vote['voter_id'] < v_id:
                            current_vote = next(vote_reader, None)
                        
                        if current_vote and current_vote.get('voter_id') == v_id:
                            voted, vote_date = 'Yes', current_vote.get('voted_date_iso', '')
                            current_vote = next(vote_reader, None)
                        
                        main_writer.writerow({
                            'Voter ID': v_id, 'Username': current_voter.get('username', ''),
                            'First Name': current_voter.get('first_name', ''), 
                            'Middle Name': current_voter.get('middle_name', ''),
                            'Last Name': current_voter.get('last_name', ''), 'Suffix': current_voter.get('suffix', ''),
                            'Area': area_name, 'Registered': current_voter.get('registered_str', 'No'),
                            'Voted': voted, 'Vote Date': vote_date
                        })
                        written_records_count += 1
                        current_voter = next(voter_reader, None)
        
        logger.info(f"Area {area_name}: Completed processing. Wrote {written_records_count} records to {area_report_filename}.")
        return area_report_filename, written_records_count, num_voters_for_area, num_votes_for_area

    except Exception as e:
        logger.error(f"Failed processing area {area_name} (ID: {area_id}): {e}", exc_info=True)
        if not os.path.exists(area_report_filename) or os.path.getsize(area_report_filename) == 0:
            try:
                with open(area_report_filename, 'w', newline='', encoding='utf-8') as af:
                    csv.DictWriter(af, fieldnames=FINAL_CSV_FIELDNAMES).writeheader()
                logger.warning(f"Created empty report for failed area: {area_report_filename}")
            except Exception as e_create:
                 logger.error(f"Could not create empty report for failed area {area_report_filename}: {e_create}")
        return area_report_filename, 0, num_voters_for_area, num_votes_for_area
    finally:
        for f_path in [temp_voters_file, temp_votes_file]:
            if f_path and os.path.exists(f_path):
                try: os.remove(f_path)
                except OSError as e_rm: logger.warning(f"Could not remove temp file {f_path}: {e_rm}")

def sort_csv_with_system_sort(input_filename: str, output_filename: str, header_fields: List[str], sort_key_names: List[str]) -> int:
    logger.info(f"Sorting '{input_filename}' by {sort_key_names} -> '{output_filename}'.")
    header_line, data_temp_path, sorted_data_temp_path = "", None, None
    data_lines_count = 0 

    try:
        with open(input_filename, 'r', newline='', encoding='utf-8') as infile:
            first_line = infile.readline()
            if not first_line: 
                logger.warning(f"Sort input '{input_filename}' is empty. Output will be empty.")
                open(output_filename, 'w').close(); return 0
            header_line = first_line.strip() # This is the actual CSV header
            
            # Skip any prepended stat lines if reading for sorting
            # However, the input_filename to sort_csv is the *concatenated* file which *only* has CSV header and data.
            # Stat lines are prepended *after* sorting. So this first_line should be the CSV header.

            actual_header_cols = [h.strip() for h in header_line.split(',')]
            sort_key_indices = []
            for key_name in sort_key_names:
                try:
                    idx = actual_header_cols.index(key_name) + 1 
                    sort_key_indices.append(f"-k{idx},{idx}f") 
                except ValueError:
                    logger.error(f"Sort key '{key_name}' not found in header of '{input_filename}'. Outputting unsorted."); shutil.copy(input_filename, output_filename)
                    with open(input_filename, 'r', newline='', encoding='utf-8') as sf: next(sf); return sum(1 for _ in sf)
            if not sort_key_indices:
                logger.error(f"No valid sort keys for '{input_filename}'. Outputting unsorted."); shutil.copy(input_filename, output_filename)
                with open(input_filename, 'r', newline='', encoding='utf-8') as sf: next(sf); return sum(1 for _ in sf)

            with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix="data_to_sort_", suffix=".csv", newline='', encoding='utf-8') as dtf:
                data_temp_path = dtf.name
                line = infile.readline()
                while line:
                    if line.strip(): dtf.write(line); data_lines_count += 1 
                    line = infile.readline()
        
        logger.info(f"Extracted {data_lines_count} data lines from '{input_filename}' for sorting.")
        if data_lines_count == 0: 
            logger.info(f"Input '{input_filename}' had no data lines. Writing header to '{output_filename}'.")
            with open(output_filename, 'w', newline='', encoding='utf-8') as of: of.write(header_line + '\n')
            return 0

        with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix="sorted_data_", suffix=".csv") as sdf:
            sorted_data_temp_path = sdf.name
        
        sort_command = ['sort', '-t', ',', *sort_key_indices, data_temp_path, '-o', sorted_data_temp_path]
        logger.debug(f"Executing sort: {' '.join(sort_command)}")
        result = subprocess.run(sort_command, capture_output=True, text=True, check=False)

        if result.returncode != 0:
            logger.error(f"Sort failed for '{data_temp_path}'. Stderr: {result.stderr}. Fallback to unsorted."); shutil.copy(input_filename, output_filename)
            return data_lines_count

        with open(output_filename, 'w', newline='', encoding='utf-8') as final_f, \
             open(sorted_data_temp_path, 'r', encoding='utf-8') as sorted_f:
            final_f.write(header_line + '\n') 
            shutil.copyfileobj(sorted_f, final_f) 
        logger.info(f"Sort successful. {data_lines_count} data lines sorted to '{output_filename}'.")
        return data_lines_count
    except FileNotFoundError: 
        logger.error(f"Sort input '{input_filename}' not found."); 
        if output_filename: open(output_filename, 'w').close()
        return 0
    except Exception as e:
        logger.error(f"Error during sort of '{input_filename}': {e}", exc_info=True)
        try:
            if input_filename and os.path.exists(input_filename) and output_filename:
                shutil.copy(input_filename, output_filename)
                logger.info(f"Fallback: Copied '{input_filename}' to '{output_filename}'.")
                with open(input_filename, 'r', newline='', encoding='utf-8') as sf: next(sf); return sum(1 for _ in sf if _.strip())
        except Exception as ce: logger.error(f"Fallback copy failed: {ce}")
        if output_filename: open(output_filename, 'w').close()
        return 0 
    finally:
        for fp in [data_temp_path, sorted_data_temp_path]:
            if fp and os.path.exists(fp):
                try: os.remove(fp)
                except OSError as e_rm: logger.warning(f"Could not remove sort temp file {fp}: {e_rm}")

def prepend_summary_stats_to_file(filepath: str, total_voters: int, total_votes: int, label: str):
    if not os.path.exists(filepath) or os.path.getsize(filepath) == 0: # Skip for non-existent or empty files
        logger.warning(f"Cannot prepend stats to non-existent or empty file: {filepath}")
        return
    summary_header = [f"# Total Voters Processed for {label}: {total_voters}\n",
                      f"# Total Votes Recorded for {label}: {total_votes}\n",
                      f"# --- CSV DATA STARTS BELOW THIS LINE ---\n"]
    temp_file_path = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', delete=False, encoding='utf-8', newline='') as tmp_f:
            temp_file_path = tmp_f.name
            for line in summary_header: tmp_f.write(line)
            with open(filepath, 'r', encoding='utf-8', newline='') as original_f:
                shutil.copyfileobj(original_f, tmp_f)
        shutil.move(temp_file_path, filepath)
        logger.info(f"Prepended summary stats for {label} to {filepath}")
    except Exception as e: logger.error(f"Error prepending stats to {filepath}: {e}")
    finally:
        if temp_file_path and os.path.exists(temp_file_path): os.remove(temp_file_path)

def main():
    parser = argparse.ArgumentParser(description='Generate voter reports for all non-test elections.')
    parser.add_argument('--output-dir', default='reports/', help='Directory to save the reports. Default: reports/')
    args = parser.parse_args()

    base_output_dir = args.output_dir
    if not os.path.exists(base_output_dir):
        os.makedirs(base_output_dir, exist_ok=True)
        logger.info(f"Created base output directory: {base_output_dir}")

    hasura_conn, keycloak_conn = None, None
    try:
        logger.info("Connecting to databases...")
        hasura_conn = connect_to_db(HASURA_DB_CONFIG)
        keycloak_conn = connect_to_db(KEYCLOAK_DB_CONFIG)

        all_elections_to_process = get_all_non_test_elections(hasura_conn, TENANT_ID, ELECTION_EVENT_ID_CONST)

        if not all_elections_to_process:
            logger.warning("No non-test elections found to process.")
            return

        total_elections_processed = 0
        for election_data in all_elections_to_process[:1]:
            election_id = election_data['id']
            election_alias = election_data.get('alias', 'NoAlias')
            election_name = election_data.get('name', 'UnnamedElection')
            
            logger.info(f"=== Processing Election ID: {election_id}, Alias: {election_alias}, Name: {election_name} ===")

            election_alias_cleaned = sanitize_filename_component(election_alias if election_alias != 'NoAlias' else election_name, 30) # Use longer name for folder
            election_specific_output_dir = os.path.join(base_output_dir, election_alias_cleaned)
            if not os.path.exists(election_specific_output_dir):
                os.makedirs(election_specific_output_dir, exist_ok=True)
            logger.info(f"Reports for this election will be in: {election_specific_output_dir}")

            election_consolidated_report_filepath = os.path.join(election_specific_output_dir, f"{election_alias_cleaned}_consolidated_report.csv")

            area_reports_details_for_election = []
            grand_total_voters_this_election = 0
            grand_total_votes_this_election = 0
            temp_unsorted_consolidated_file = None

            areas = get_areas_by_election_id(hasura_conn, TENANT_ID, ELECTION_EVENT_ID_CONST, election_id)
            logger.info(f"Found {len(areas)} areas for election '{election_name}'.")
            
            if not areas:
                logger.warning(f"No areas for election '{election_name}'. Consolidated report will be empty (header only).")
                with open(election_consolidated_report_filepath, 'w', newline='', encoding='utf-8') as f_out:
                    csv.DictWriter(f_out, fieldnames=FINAL_CSV_FIELDNAMES).writeheader()
            else:
                for i, area_data in enumerate(areas):
                    area_name_for_label = area_data.get('name', f"Area{i+1}")
                    logger.info(f"--- Processing Area {i+1}/{len(areas)}: {area_name_for_label} (Election: {election_name}) ---")
                    
                    area_filepath, _, area_voters, area_votes = process_area(
                        area_data, TENANT_ID, ELECTION_EVENT_ID_CONST,
                        REALM_NAME, election_id,
                        election_specific_output_dir, # Pass election specific dir
                        hasura_conn, keycloak_conn
                    )
                    if area_filepath and os.path.exists(area_filepath):
                        area_reports_details_for_election.append((area_filepath, area_voters, area_votes, area_name_for_label))
                        grand_total_voters_this_election += area_voters
                        grand_total_votes_this_election += area_votes
                    else:
                        logger.error(f"Failed to process or get valid file for area {area_name_for_label}, skipping its inclusion.")

                if area_reports_details_for_election:
                    logger.info(f"Consolidating {len(area_reports_details_for_election)} area reports for election '{election_name}'...")
                    try:
                        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='_consolidated_unsorted.csv', newline='', encoding='utf-8') as tmp_f:
                            temp_unsorted_consolidated_file = tmp_f.name
                        
                        with open(temp_unsorted_consolidated_file, 'w', newline='', encoding='utf-8') as consolidated_f:
                            consolidated_writer = csv.writer(consolidated_f)
                            consolidated_writer.writerow(FINAL_CSV_FIELDNAMES)
                            total_data_rows_concatenated = 0
                            for area_filepath, _, _, _ in area_reports_details_for_election:
                                try:
                                    with open(area_filepath, 'r', newline='', encoding='utf-8') as area_f:
                                        area_reader = csv.reader(area_f)
                                        # Skip prepended stats & header
                                        for _ in range(4): # 3 stat lines + 1 header line
                                            try: next(area_reader)
                                            except StopIteration: break 
                                        
                                        rows_in_area_file = 0
                                        for row in area_reader:
                                            if row: consolidated_writer.writerow(row); rows_in_area_file +=1
                                        logger.debug(f"Concatenated {rows_in_area_file} data rows from {area_filepath}")
                                        total_data_rows_concatenated += rows_in_area_file
                                except Exception as e_concat_file:
                                    logger.error(f"Error reading/concatenating {area_filepath}: {e_concat_file}")
                            logger.info(f"Total data rows in '{temp_unsorted_consolidated_file}': {total_data_rows_concatenated}")

                        sort_csv_with_system_sort(
                            temp_unsorted_consolidated_file, election_consolidated_report_filepath, 
                            FINAL_CSV_FIELDNAMES, SYSTEM_SORT_PRIMARY_KEYS
                        )
                    finally:
                        if temp_unsorted_consolidated_file and os.path.exists(temp_unsorted_consolidated_file):
                            os.remove(temp_unsorted_consolidated_file)
                else: # No successful area reports to consolidate
                    logger.warning(f"No area reports to consolidate for election '{election_name}'. Creating empty consolidated report (header only).")
                    with open(election_consolidated_report_filepath, 'w', newline='', encoding='utf-8') as f_out:
                        csv.DictWriter(f_out, fieldnames=FINAL_CSV_FIELDNAMES).writeheader()

            logger.info(f"Prepending statistics for election '{election_name}' reports...")
            for area_filepath, area_voters, area_votes, area_name_label in area_reports_details_for_election:
                prepend_summary_stats_to_file(area_filepath, area_voters, area_votes, f"Area '{area_name_label}' (Election: {election_alias})")
            
            prepend_summary_stats_to_file(election_consolidated_report_filepath, grand_total_voters_this_election, grand_total_votes_this_election, f"Election '{election_alias}'")
            
            logger.info(f"--- Finished processing election: {election_alias} ---")
            total_elections_processed += 1

        logger.info(f"All {total_elections_processed} elections processed.")

    except Exception as e: 
        logger.critical(f"A critical error occurred in main execution: {e}", exc_info=True)
        sys.exit(1)
    finally:
        if hasura_conn: hasura_conn.close()
        if keycloak_conn: keycloak_conn.close()
        logger.info("Database connections closed.")

if __name__ == '__main__':
    main()