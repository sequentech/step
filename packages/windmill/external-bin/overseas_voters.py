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

AREA_ID_ATTR_NAME = "area-id"  # As per user's script
VALIDATE_ID_ATTR_NAME = "validate_id"
VALIDATE_ID_REGISTERED_VOTER = "registered-voter"

FINAL_CSV_FIELDNAMES = [
    'Voter ID', 'Username', 'First Name', 'Middle Name', 'Last Name',
    'Suffix', 'Area', 'Registered', 'Voted', 'Vote Date'
]
# Sort order: Last Name, First Name, Middle Name
SYSTEM_SORT_PRIMARY_KEYS = ['Last Name', 'First Name', 'Middle Name']

# --- Logging Setup ---
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# --- Database Configuration ---
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

# --- Helper Functions ---

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
    name = str(name).lower() # Ensure it's a string
    name = re.sub(r'\s+', '_', name)
    name = re.sub(r'[^\w\-_.]', '', name)
    name = name[:max_length]
    return name.rstrip('-_.') or "sanitized_name"


# --- Database Query Functions ---
def get_election_by_alias_pattern(hasura_conn, tenant_id: str, election_event_id: str, alias_pattern: str) -> Optional[Dict]:
    sql_like_pattern = alias_pattern
    if '%' not in sql_like_pattern and '_' not in sql_like_pattern:
        sql_like_pattern = f"%{alias_pattern}%"
    logger.info(f"Searching for election with alias pattern: '{sql_like_pattern}'")
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT id, alias, name
            FROM sequent_backend.election
            WHERE tenant_id = %s AND election_event_id = %s AND alias ILIKE %s
            ORDER BY alias, id
            LIMIT 1
        """, (tenant_id, election_event_id, sql_like_pattern))
        return cursor.fetchone()

def get_areas_by_election_id(hasura_conn, tenant_id: str, election_event_id: str, election_id: str) -> List[Dict]:
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT DISTINCT ON (a.id) a.* FROM sequent_backend.area a
            JOIN sequent_backend.area_contest ac ON a.id = ac.area_id AND a.election_event_id = ac.election_event_id AND a.tenant_id = ac.tenant_id
            JOIN sequent_backend.contest c ON ac.contest_id = c.id AND ac.election_event_id = c.election_event_id AND ac.tenant_id = c.tenant_id
            WHERE c.tenant_id = %s AND c.election_event_id = %s AND c.election_id = %s
            ORDER BY a.id -- Consistent ordering of areas
        """, (tenant_id, election_event_id, election_id))
        return cursor.fetchall()

# --- File Dumping Helper Functions using COPY ---

def dump_all_voters_to_file_copy(keycloak_conn, realm_name: str, area_id_value: str, output_filepath: str) -> int:
    logger.info(f"Dumping all voters for area {area_id_value} to {output_filepath} using COPY")
    
    # This SQL needs to match the structure expected by the merge process later
    # It should output: 'id', 'username', 'first_name', 'middle_name', 'last_name', 'suffix', 'registered_str'
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
    
    # The COPY command itself will use the column names from the SELECT query as the header.
    # The order of columns in this SELECT query must match the `voter_fieldnames` used by DictReader in `process_area`
    # if we stick to DictReader. If we use plain csv.reader, order is by index.
    # For simplicity, let's ensure this intermediate voters file uses simple fieldnames for now.
    # The `process_area` logic needs to be aware of these fieldnames.

    # Fieldnames for the temporary voter file, matching the COPY output.
    # These are derived from the SELECT query above.
    voter_copy_fieldnames = ['id', 'username', 'first_name', 'middle_name', 'last_name', 'suffix', 'registered_str']


    rows_dumped = 0
    try:
        with open(output_filepath, 'wb') as f_voters: # copy_expert needs binary mode for STDOUT
            with keycloak_conn.cursor() as cursor:
                # Construct the full COPY command
                # The header from the SQL query aliases will be used by COPY CSV HEADER
                copy_command = f"COPY ({sql_query.replace('%s', '%(realm_name)s').replace('%s', '%(area_id)s', 1)}) TO STDOUT WITH (FORMAT CSV, HEADER TRUE)"
                
                # It's safer to ensure placeholders are correctly handled by psycopg2 if they were inside the main query
                # However, copy_expert's SQL parameter is for the COPY command itself, not for parameterizing the subquery easily.
                # A more robust way for subquery parameterization:
                # cursor.mogrify(sub_query, params) and then embed the mogrified query.
                # For now, direct replacement (carefully done) or pre-mogrified subquery.
                
                # Let's use mogrify for safety for the subquery part
                sub_query_for_copy = cursor.mogrify(sql_query, (realm_name, area_id_value)).decode(keycloak_conn.encoding)
                final_copy_command = f"COPY ({sub_query_for_copy}) TO STDOUT WITH (FORMAT CSV, HEADER TRUE)"

                cursor.copy_expert(final_copy_command, f_voters)
                rows_dumped = cursor.rowcount
        logger.info(f"Finished dumping {rows_dumped} voters for area {area_id_value} using COPY.")
    except Exception as e:
        logger.error(f"Error dumping voters for area {area_id_value} using COPY: {e}", exc_info=True)
        raise # Or handle by returning 0 and an error status
    return rows_dumped


def dump_all_cast_votes_to_file_copy(
    hasura_conn,
    tenant_id: str,
    election_event_id: str,
    election_id: str,
    area_id_value: str,
    output_filepath: str) -> int:
    logger.info(f"Dumping all cast votes for area {area_id_value} to {output_filepath} using COPY")

    # Outputs: 'voter_id', 'voted_date_iso'
    sql_query = f"""
    WITH RankedVotes AS (
        SELECT
            v.voter_id_string,
            v.created_at AS voted_date,
            ROW_NUMBER() OVER(PARTITION BY v.voter_id_string ORDER BY v.created_at DESC) as rn
        FROM sequent_backend.cast_vote v
        JOIN sequent_backend.election e ON v.election_id = e.id
            AND v.tenant_id = e.tenant_id 
            AND v.election_event_id = e.election_event_id
        WHERE
            v.tenant_id = %s AND
            v.election_event_id = %s AND
            v.election_id = %s AND
            v.area_id = %s
    )
    SELECT
        voter_id_string AS voter_id,
        to_char(voted_date, 'YYYY-MM-DD"T"HH24:MI:SS.USOF') AS voted_date_iso
    FROM RankedVotes
    WHERE rn = 1
    ORDER BY voter_id_string
    """
    # Fieldnames for the temporary votes file, matching the COPY output.
    vote_copy_fieldnames = ['voter_id', 'voted_date_iso']

    rows_dumped = 0
    try:
        with open(output_filepath, 'wb') as f_votes: # copy_expert needs binary mode
            with hasura_conn.cursor() as cursor:
                sub_query_for_copy = cursor.mogrify(sql_query, (tenant_id, election_event_id, election_id, area_id_value)).decode(hasura_conn.encoding)
                final_copy_command = f"COPY ({sub_query_for_copy}) TO STDOUT WITH (FORMAT CSV, HEADER TRUE)"
                cursor.copy_expert(final_copy_command, f_votes)
                rows_dumped = cursor.rowcount
        logger.info(f"Finished dumping {rows_dumped} cast votes for area {area_id_value} using COPY.")
    except Exception as e:
        logger.error(f"Error dumping cast votes for area {area_id_value} using COPY: {e}", exc_info=True)
        raise
    return rows_dumped

# --- Core Processing Function ---
def process_area(
    area_dict: Dict,
    tenant_id: str,
    election_event_id: str,
    keycloak_realm_name: str,
    election_id: str,
    election_alias_cleaned: str, # For filename
    output_dir: str, # Directory for area file
    hasura_conn,
    keycloak_conn
) -> Tuple[Optional[str], int, int, int]:
    area_id = area_dict['id']
    area_name = area_dict.get('name', 'UnknownArea')
    area_name_cleaned = sanitize_filename_component(area_name, 20)
    
    area_report_filename = os.path.join(output_dir, f"election_{election_alias_cleaned}_area_{area_name_cleaned}_report.csv")

    logger.info(f"Processing area: {area_name} (ID: {area_id}). Output will be: {area_report_filename}")

    temp_voters_file, temp_votes_file = None, None
    num_voters_for_area = 0
    num_votes_for_area = 0
    written_records_count = 0

    try:
        # Create uniquely named temp files for this area's raw dumps
        with tempfile.NamedTemporaryFile(mode='wb', delete=False, prefix=f"area_{area_id}_voters_raw_", suffix='.csv') as tmp_f_v:
            temp_voters_file = tmp_f_v.name
        # `dump_all_voters_to_file_copy` writes in binary mode, so temp file is opened in 'wb'

        with tempfile.NamedTemporaryFile(mode='wb', delete=False, prefix=f"area_{area_id}_votes_raw_", suffix='.csv') as tmp_f_c:
            temp_votes_file = tmp_f_c.name

        num_voters_for_area = dump_all_voters_to_file_copy(keycloak_conn, keycloak_realm_name, area_id, temp_voters_file)
        num_votes_for_area = dump_all_cast_votes_to_file_copy(hasura_conn, tenant_id, election_event_id, election_id, area_id, temp_votes_file)
        
        logger.info(f"Area {area_name} (ID: {area_id}): Dumped {num_voters_for_area} voters and {num_votes_for_area} cast vote records to temporary files using COPY.")

        # Fieldnames for the intermediate files (as output by COPY)
        # These must match the SELECT aliases in the COPY subqueries
        voter_copy_fieldnames = ['id', 'username', 'first_name', 'middle_name', 'last_name', 'suffix', 'registered_str']
        vote_copy_fieldnames = ['voter_id', 'voted_date_iso']


        if num_voters_for_area == 0 and num_votes_for_area == 0 : # if area is completely empty based on dumps
             logger.info(f"No voters or votes found for area {area_name} (ID: {area_id}). Creating empty report file (header only).")
             with open(area_report_filename, 'w', newline='', encoding='utf-8') as area_csv_f:
                main_writer = csv.DictWriter(area_csv_f, fieldnames=FINAL_CSV_FIELDNAMES)
                main_writer.writeheader()
             # written_records_count remains 0
        else:
            # Open raw dump files in text mode for csv.DictReader
            with open(area_report_filename, 'w', newline='', encoding='utf-8') as area_csv_f, \
                 open(temp_voters_file, 'r', newline='', encoding='utf-8') as voters_f, \
                 open(temp_votes_file, 'r', newline='', encoding='utf-8') as votes_f:

                main_writer = csv.DictWriter(area_csv_f, fieldnames=FINAL_CSV_FIELDNAMES)
                main_writer.writeheader()

                # Use the specific fieldnames from the COPY output for these readers
                voter_reader = csv.DictReader(voters_f, fieldnames=voter_copy_fieldnames)
                vote_reader = csv.DictReader(votes_f, fieldnames=vote_copy_fieldnames)
                
                # Skip headers from raw files if DictReader doesn't handle it (it does if fieldnames not passed, but here we pass fieldnames)
                next(voter_reader) # Skip header row from COPY output file
                if num_votes_for_area > 0 : # only skip header if votes file has content (and thus a header)
                    next(vote_reader)  # Skip header row from COPY output file

                current_voter = next(voter_reader, None)
                current_vote = next(vote_reader, None) if num_votes_for_area > 0 else None


                while current_voter:
                    v_id = current_voter['id']
                    voted, vote_date = 'No', ''
                    
                    # Ensure current_vote['voter_id'] is compared correctly if it exists
                    while current_vote and current_vote.get('voter_id') and current_vote['voter_id'] < v_id:
                        current_vote = next(vote_reader, None)
                    
                    if current_vote and current_vote.get('voter_id') == v_id:
                        voted = 'Yes'
                        vote_date = current_vote.get('voted_date_iso', '')
                        current_vote = next(vote_reader, None)
                    
                    main_writer.writerow({
                        'Voter ID': v_id, 
                        'Username': current_voter.get('username', ''),
                        'First Name': current_voter.get('first_name', ''), 
                        'Middle Name': current_voter.get('middle_name', ''),
                        'Last Name': current_voter.get('last_name', ''), 
                        'Suffix': current_voter.get('suffix', ''),
                        'Area': area_name, 
                        'Registered': current_voter.get('registered_str', 'No'),
                        'Voted': voted, 
                        'Vote Date': vote_date
                    })
                    written_records_count += 1
                    current_voter = next(voter_reader, None)
        
        logger.info(f"Completed processing area: {area_name} (ID: {area_id}). Wrote {written_records_count} records to {area_report_filename}.")
        return area_report_filename, written_records_count, num_voters_for_area, num_votes_for_area

    except Exception as e:
        logger.error(f"Failed processing area {area_name} (ID: {area_id}): {e}", exc_info=True)
        # Create an empty file or file with error message? For now, return None path.
        # Ensure an empty file is created so subsequent append logic doesn't fail.
        if not os.path.exists(area_report_filename) or os.path.getsize(area_report_filename) == 0:
            try:
                with open(area_report_filename, 'w', newline='', encoding='utf-8') as area_csv_f:
                    writer = csv.DictWriter(area_csv_f, fieldnames=FINAL_CSV_FIELDNAMES)
                    writer.writeheader() # Write at least a header
                logger.warning(f"Created empty (header only) report for failed area: {area_report_filename}")
            except Exception as e_create:
                 logger.error(f"Could not even create empty report for failed area {area_report_filename}: {e_create}")

        return area_report_filename, 0, num_voters_for_area, num_votes_for_area # Return counts up to failure point
    finally:
        for f_path in [temp_voters_file, temp_votes_file]:
            if f_path and os.path.exists(f_path):
                try:
                    os.remove(f_path)
                    logger.debug(f"Removed temp raw dump file: {f_path}")
                except OSError as e_remove:
                    logger.warning(f"Could not remove temp raw dump file {f_path}: {e_remove}")
    

# --- Final Output Sorting Function (Using System `sort`) ---
# (sort_csv_with_system_sort remains largely the same as in the previous version,
#  as it operates on a single input file to produce a single sorted output file)
def sort_csv_with_system_sort(input_filename: str, output_filename: str, header_fields: List[str], sort_key_names: List[str]) -> int:
    logger.info(f"Sorting '{input_filename}' by {sort_key_names} using system sort -> '{output_filename}'.")
    header_line = ""
    data_temp_path = None
    sorted_data_temp_path = None
    data_lines_count = 0 

    try:
        with open(input_filename, 'r', newline='', encoding='utf-8') as infile:
            first_line = infile.readline()
            if not first_line: 
                logger.warning(f"Input file '{input_filename}' is empty for sorting. Output will be empty.")
                open(output_filename, 'w').close() 
                return 0
            
            header_line = first_line.strip()
            actual_header_cols = [h.strip() for h in header_line.split(',')] 

            sort_key_indices = []
            for key_name in sort_key_names:
                try:
                    idx = actual_header_cols.index(key_name) + 1 
                    sort_key_indices.append(f"-k{idx},{idx}f") 
                except ValueError:
                    logger.error(f"Sort key '{key_name}' not found in header: {actual_header_cols} of '{input_filename}'. Outputting unsorted.")
                    shutil.copy(input_filename, output_filename)
                    copied_lines = 0
                    with open(input_filename, 'r', newline='', encoding='utf-8') as src_f:
                        next(src_f, None) 
                        for _ in src_f: copied_lines +=1
                    return copied_lines
            
            if not sort_key_indices:
                logger.error(f"No valid sort keys derived for '{input_filename}'. Outputting unsorted.")
                shutil.copy(input_filename, output_filename)
                copied_lines = 0
                with open(input_filename, 'r', newline='', encoding='utf-8') as src_f:
                    next(src_f, None) 
                    for _ in src_f: copied_lines +=1
                return copied_lines

            with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix="data_to_sort_", suffix=".csv", newline='', encoding='utf-8') as data_temp_file:
                data_temp_path = data_temp_file.name
                line = infile.readline()
                while line:
                    if line.strip() and not line.startswith("#"): # Skip empty lines or comment lines
                        data_temp_file.write(line)
                        data_lines_count += 1 
                    line = infile.readline()
        
        logger.info(f"Extracted {data_lines_count} data lines from '{input_filename}' to '{data_temp_path}' for sorting.")
        
        if data_lines_count == 0: 
            logger.info(f"Input file '{input_filename}' contained only a header or no data lines for sorting. Writing header to '{output_filename}'.")
            with open(output_filename, 'w', newline='', encoding='utf-8') as outfile:
                outfile.write(header_line + '\n')
            return 0

        with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix="sorted_data_", suffix=".csv", newline='', encoding='utf-8') as sorted_data_file:
            sorted_data_temp_path = sorted_data_file.name

        sort_command = ['sort', '-t', ',']
        sort_command.extend(sort_key_indices)
        sort_command.extend([data_temp_path, '-o', sorted_data_temp_path])

        logger.debug(f"Executing sort command: {' '.join(sort_command)}")
        result = subprocess.run(sort_command, capture_output=True, text=True, check=False)

        if result.returncode != 0:
            logger.error(f"System sort command failed for '{data_temp_path}'. Stderr: {result.stderr}. Stdout: {result.stdout}")
            logger.error(f"Outputting {data_lines_count} unsorted data lines from '{input_filename}' as a fallback to '{output_filename}'.")
            shutil.copy(input_filename, output_filename)
            return data_lines_count

        with open(output_filename, 'w', newline='', encoding='utf-8') as final_outfile:
            final_outfile.write(header_line + '\n') 
            with open(sorted_data_temp_path, 'r', encoding='utf-8') as sorted_data_infile:
                shutil.copyfileobj(sorted_data_infile, final_outfile) 
        
        logger.info(f"System sort successful. {data_lines_count} data lines sorted. Sorted report saved to '{output_filename}'.")
        return data_lines_count

    except FileNotFoundError: 
        logger.error(f"Input file for sorting '{input_filename}' not found.")
        if output_filename: open(output_filename, 'w').close() # create empty output
        return 0
    except Exception as e:
        logger.error(f"An error occurred during system sort of '{input_filename}': {e}", exc_info=True)
        try:
            if input_filename and os.path.exists(input_filename) and output_filename:
                shutil.copy(input_filename, output_filename)
                logger.info(f"Fallback: Copied original content of '{input_filename}' to '{output_filename}' due to sort error.")
                # Re-count data lines from input_filename if copy succeeded
                copied_data_lines = 0
                with open(input_filename, 'r', newline='', encoding='utf-8') as src_f:
                    next(src_f, None) # Skip header
                    for line in src_f:
                        if line.strip() and not line.startswith("#"):
                             copied_data_lines +=1
                return copied_data_lines
        except Exception as copy_e:
            logger.error(f"Fallback copy also failed: {copy_e}")
        if output_filename: open(output_filename, 'w').close() # create empty output
        return 0 
    finally:
        for f_path in [data_temp_path, sorted_data_temp_path]:
            if f_path and os.path.exists(f_path):
                try:
                    os.remove(f_path)
                    logger.debug(f"Removed sort temp file: {f_path}")
                except Exception as e_remove:
                    logger.warning(f"Could not remove sort temp file {f_path}: {e_remove}")

def prepend_summary_stats_to_file(filepath: str, total_voters: int, total_votes: int, label: str):
    if not os.path.exists(filepath):
        logger.warning(f"Cannot prepend stats, file not found: {filepath}")
        return

    summary_header_lines = [
        f"# Total Voters Processed for {label}: {total_voters}\n",
        f"# Total Votes Recorded for {label}: {total_votes}\n",
        f"# --- CSV DATA STARTS BELOW THIS LINE ---\n" # Optional: a clearer separator
    ]

    temp_file_path = None
    try:
        # Create a temporary file
        with tempfile.NamedTemporaryFile(mode='w', delete=False, encoding='utf-8', newline='') as tmp_f:
            temp_file_path = tmp_f.name

            # 1. Write the summary stats to the temp file
            for line in summary_header_lines:
                tmp_f.write(line)

            # 2. Append the original file's content to the temp file
            with open(filepath, 'r', encoding='utf-8', newline='') as original_f:
                # You might want to skip any pre-existing comment lines if this function could be called multiple times
                # For now, let's assume it's clean CSV data or the prepended stats are idempotent.
                shutil.copyfileobj(original_f, tmp_f)

        # 3. Replace the original file with the temporary file
        shutil.move(temp_file_path, filepath)
        logger.info(f"Prepended summary stats for {label} to {filepath}")
        temp_file_path = None # So it's not removed in finally if move was successful
    except Exception as e:
        logger.error(f"Error prepending stats to {filepath} for {label}: {e}")
    finally:
        if temp_file_path and os.path.exists(temp_file_path):
            os.remove(temp_file_path) # Clean up temp file if something went wrong

# --- Main Execution ---
def main():
    parser = argparse.ArgumentParser(description='Generate voter reports per area and a consolidated, sorted election report.')
    parser.add_argument('election_alias_pattern', help='Pattern for ILIKE matching election alias.')
    parser.add_argument('--output-dir', default='.', help='Final consolidated sorted output CSV file name.')
    args = parser.parse_args()

    hasura_conn, keycloak_conn = None, None
    
    # Determine output directory for area files
    output_dir = args.output_dir
    if not output_dir: # If args.output is just a filename, use current dir
        output_dir = '.'
    elif not os.path.exists(output_dir):
        os.makedirs(output_dir, exist_ok=True)
        logger.info(f"Created output directory: {output_dir}")

    area_reports_details = [] # List of tuples: (filepath, area_voters, area_votes, area_name_label)
    grand_total_records_written_consolidated = 0 # from final sorted file
    grand_total_voters_election = 0
    grand_total_votes_election = 0
    
    temp_unsorted_consolidated_file = None

    try:
        logger.info("Connecting to databases...")
        hasura_conn = connect_to_db(HASURA_DB_CONFIG)
        keycloak_conn = connect_to_db(KEYCLOAK_DB_CONFIG)

        found_election = get_election_by_alias_pattern(
            hasura_conn, TENANT_ID, ELECTION_EVENT_ID_CONST, args.election_alias_pattern
        )

        if not found_election:
            logger.error(f"No election found matching alias pattern '{args.election_alias_pattern}'. Process aborted.")
            sys.exit(1)
        
        logger.info(f"Found 1 election to process.")
        found_election_id = found_election['id']
        found_election_alias = found_election.get('alias', 'UnknownElection')
        found_election_name = found_election.get('name', 'N/A')
        election_alias_cleaned = sanitize_filename_component(found_election_alias, 20)
        election_csv_filename = os.path.join(output_dir, f"election_{election_alias_cleaned}_report.csv")
        logger.info(f"Election Output will be: {election_csv_filename}")

        print(f"Processing Election: ID='{found_election_id}', Alias='{found_election_alias}', Name='{found_election_name}'")
        logger.info(f"Processing Election: ID='{found_election_id}', Alias='{found_election_alias}', Name='{found_election_name}'")

        areas = get_areas_by_election_id(hasura_conn, TENANT_ID, ELECTION_EVENT_ID_CONST, found_election_id)
        logger.info(f"Found {len(areas)} areas for this election.")
        
        if not areas:
            logger.warning(f"No areas found for election ID: {found_election_id}. Final report will likely be empty or header only.")
        else:
            for i, area_data in enumerate(areas):
                area_name_for_label = area_data.get('name', f"Area{i+1}")
                logger.info(f"--- Processing Area {i+1}/{len(areas)}: {area_name_for_label} (ID: {area_data.get('id')}) ---")
                
                area_filepath, _, area_voters, area_votes = process_area(
                    area_data, TENANT_ID, ELECTION_EVENT_ID_CONST,
                    REALM_NAME, found_election_id,
                    election_alias_cleaned, output_dir,
                    hasura_conn, keycloak_conn
                )
                if area_filepath: # If processing didn't completely fail for the area file path
                    area_reports_details.append((area_filepath, area_voters, area_votes, area_name_for_label))
                    grand_total_voters_election += area_voters
                    grand_total_votes_election += area_votes
                else:
                    logger.error(f"Failed to get a valid filepath for area {area_name_for_label}, skipping its inclusion.")

        # Consolidate area files if any were processed
        if area_reports_details:
            logger.info(f"Consolidating {len(area_reports_details)} area reports...")
            with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='_consolidated_unsorted.csv', newline='', encoding='utf-8') as tmp_f:
                temp_unsorted_consolidated_file = tmp_f.name
            
            with open(temp_unsorted_consolidated_file, 'w', newline='', encoding='utf-8') as consolidated_f:
                consolidated_writer = csv.writer(consolidated_f)
                
                # Write header from FINAL_CSV_FIELDNAMES
                consolidated_writer.writerow(FINAL_CSV_FIELDNAMES)
                
                total_data_rows_concatenated = 0
                for i, (area_filepath, _, _, _) in enumerate(area_reports_details):
                    try:
                        with open(area_filepath, 'r', newline='', encoding='utf-8') as area_f:
                            area_reader = csv.reader(area_f)
                            next(area_reader) # Skip header of the area file
                            rows_in_area_file = 0
                            for row in area_reader:
                                if row: # Ensure row is not empty
                                    consolidated_writer.writerow(row)
                                    rows_in_area_file +=1
                            logger.info(f"Concatenated {rows_in_area_file} data rows from {area_filepath}")
                            total_data_rows_concatenated += rows_in_area_file
                    except Exception as e_concat:
                        logger.error(f"Error concatenating {area_filepath}: {e_concat}")
                logger.info(f"Finished consolidating. Total data rows in unsorted consolidated file '{temp_unsorted_consolidated_file}': {total_data_rows_concatenated}")

            logger.info("Sorting the consolidated report...")
            grand_total_records_written_consolidated = sort_csv_with_system_sort(
                temp_unsorted_consolidated_file, election_csv_filename, FINAL_CSV_FIELDNAMES, SYSTEM_SORT_PRIMARY_KEYS
            )
            logger.info(f"Successfully generated sorted consolidated report: {election_csv_filename} with {grand_total_records_written_consolidated} data records.")
        else:
            logger.warning("No area reports were generated or successfully processed. Creating an empty final report.")
            with open(election_csv_filename, 'w', newline='', encoding='utf-8') as f_out:
                writer = csv.DictWriter(f_out, fieldnames=FINAL_CSV_FIELDNAMES)
                writer.writeheader() # Write header to empty file
            grand_total_records_written_consolidated = 0

        # Append stats to individual area files
        logger.info("Appending statistics to individual area reports...")
        for area_filepath, area_voters, area_votes, area_name_label in area_reports_details:
            prepend_summary_stats_to_file(area_filepath, area_voters, area_votes, f"Area '{area_name_label}'")

        # Append stats to the final consolidated report
        logger.info(f"Appending overall statistics to the final report: {election_csv_filename}")
        prepend_summary_stats_to_file(election_csv_filename, grand_total_voters_election, grand_total_votes_election, f"Election '{found_election_alias}'")

        logger.info("All processing finished.")

    except Exception as e: 
        logger.error(f"A critical error occurred in main execution: {e}", exc_info=True)
        sys.exit(1)
    finally:
        if hasura_conn: hasura_conn.close()
        if keycloak_conn: keycloak_conn.close()
        logger.info("Database connections closed.")
        if temp_unsorted_consolidated_file and os.path.exists(temp_unsorted_consolidated_file):
            try:
                os.remove(temp_unsorted_consolidated_file)
                logger.info(f"Removed temporary unsorted consolidated file: {temp_unsorted_consolidated_file}")
            except OSError as e_remove_main_temp:
                 logger.warning(f"Could not remove temporary consolidated file {temp_unsorted_consolidated_file}: {e_remove_main_temp}")


if __name__ == '__main__':
    main()
