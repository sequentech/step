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

# --- Database Query Functions ---
def get_election_by_alias_pattern(hasura_conn, tenant_id: str, election_event_id: str, alias_pattern: str) -> Optional[Dict]:
    """Get the first election whose alias matches the pattern (ILIKE), ordered deterministically."""
    sql_like_pattern = alias_pattern
    if '%' not in sql_like_pattern and '_' not in sql_like_pattern:
        sql_like_pattern = f"%{alias_pattern}%"
    
    logger.info(f"Searching for election with alias pattern: '{sql_like_pattern}'")
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT id, alias, name
            FROM sequent_backend.election
            WHERE tenant_id = %s AND election_event_id = %s AND alias ILIKE %s
            ORDER BY alias, id -- Deterministic ordering for "first"
            LIMIT 1
        """, (tenant_id, election_event_id, sql_like_pattern))
        row = cursor.fetchone()
        return row

def get_areas_by_election_id(hasura_conn, tenant_id: str, election_event_id: str, election_id: str) -> List[Dict]:
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT DISTINCT ON (a.id) a.* FROM sequent_backend.area a
            JOIN sequent_backend.area_contest ac ON a.id = ac.area_id AND a.election_event_id = ac.election_event_id AND a.tenant_id = ac.tenant_id
            JOIN sequent_backend.contest c ON ac.contest_id = c.id AND ac.election_event_id = c.election_event_id AND ac.tenant_id = c.tenant_id
            WHERE c.tenant_id = %s AND c.election_event_id = %s AND c.election_id = %s
        """, (tenant_id, election_event_id, election_id))
        return cursor.fetchall()


def get_non_test_elections(hasura_conn, tenant_id: str, election_event_id: str) -> List[Dict]:
    logger.info(f"getting non test election for tenant_id={tenant_id} and election_event_id={election_event_id}")
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute(f"""
            SELECT id FROM sequent_backend.election
            WHERE tenant_id = '{tenant_id}' AND election_event_id = '{election_event_id}'
            AND name NOT ILIKE '%Test%' AND (alias IS NULL OR alias NOT ILIKE '%Test%')
        """)
        return cursor.fetchall()


def get_voters_by_area_id(keycloak_conn, realm_name: str, area_id_value: str,
                          batch_size: int = 1000, offset: int = 0) -> Tuple[List[Dict], Optional[int]]:
    with keycloak_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        sql = f"""
        SELECT
            u.id, u.first_name, u.last_name, u.username,
            COALESCE(attr_json.attributes ->> 'middleName', '') AS middle_name,
            COALESCE(attr_json.attributes ->> 'suffix', '') AS suffix,
            COALESCE(attr_json.attributes ->> '{VALIDATE_ID_ATTR_NAME}', '') AS validate_id,
            COUNT(u.id) OVER() AS total_count
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
        LIMIT %s OFFSET %s
        """
        cursor.execute(sql, (realm_name, area_id_value, batch_size, offset))
        rows = cursor.fetchall()
        voters = [{
            'id': row['id'], 'first_name': row.get('first_name'), 'last_name': row.get('last_name'),
            'username': row.get('username'), 'middle_name': row.get('middle_name'),
            'suffix': row.get('suffix'),
            'registered': row.get('validate_id') == VALIDATE_ID_REGISTERED_VOTER
        } for row in rows]
        total_count = int(rows[0]['total_count']) if rows else 0
        next_offset = offset + len(rows) if offset + len(rows) < total_count else None
        return voters, next_offset

def get_cast_votes(
    hasura_conn,
    tenant_id: str,
    election_event_id: str,
    election_id: str,
    area_id: str,
    batch_size: int = 1000,
    offset: int = 0
) -> List[Dict]:
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        sql = f"""
        WITH RankedVotes AS (
            SELECT
                v.voter_id_string, v.created_at AS voted_date,
                ROW_NUMBER() OVER(PARTITION BY v.voter_id_string ORDER BY v.created_at DESC) as rn
            FROM sequent_backend.cast_vote v
            JOIN sequent_backend.election e ON v.election_id = e.id
                AND v.tenant_id = e.tenant_id 
                AND v.election_event_id = e.election_event_id
            WHERE
                v.tenant_id = '{tenant_id}' AND
                v.election_event_id = '{election_event_id}' AND
                v.election_id = '{election_id}' AND
                v.area_id = '{area_id}'
        )
        SELECT voter_id_string, voted_date FROM RankedVotes
        WHERE rn = 1 ORDER BY voter_id_string LIMIT {batch_size} OFFSET {offset}
        """
        cursor.execute(sql, ())
        return cursor.fetchall()


# --- File Dumping Helper Functions ---
def dump_all_voters_to_file(keycloak_conn, realm_name: str, area_id_value: str, temp_voter_filename: str) -> int:
    logger.info(f"Dumping all voters for area {area_id_value} to {temp_voter_filename}")
    total_voters_dumped = 0
    with open(temp_voter_filename, 'w', newline='', encoding='utf-8') as f_voters:
        voter_writer = csv.writer(f_voters)
        voter_writer.writerow(['id', 'username', 'first_name', 'middle_name', 'last_name', 'suffix', 'registered_str'])
        offset, batch_size = 0, 1000
        while True:
            voters_batch, next_offset = get_voters_by_area_id(
                keycloak_conn, realm_name, area_id_value, batch_size, offset)
            if not voters_batch: break
            
            batch_dump_count = 0
            for v_data in voters_batch:
                voter_writer.writerow([
                    v_data['id'], v_data.get('username', ''), v_data.get('first_name', ''),
                    v_data.get('middle_name', ''), v_data.get('last_name', ''),
                    v_data.get('suffix', ''), 'Yes' if v_data.get('registered') else 'No'])
                batch_dump_count += 1
            total_voters_dumped += batch_dump_count
            logger.info(f"Dumped {batch_dump_count} voters (batch) for area {area_id_value}. Total dumped for this area so far: {total_voters_dumped}")

            if not next_offset: break
            offset = next_offset
    logger.info(f"Finished dumping voters for area {area_id_value}. Total voters dumped: {total_voters_dumped}.")
    return total_voters_dumped


def dump_all_cast_votes_to_file(
    hasura_conn,
    tenant_id: str,
    election_event_id: str,
    election_id: str,
    area_id_value: str,
    temp_vote_filename: str) -> int:
    logger.info(f"Dumping all cast votes for area {area_id_value} to {temp_vote_filename}")
    total_votes_dumped = 0
    with open(temp_vote_filename, 'w', newline='', encoding='utf-8') as f_votes:
        vote_writer = csv.writer(f_votes)
        vote_writer.writerow(['voter_id', 'voted_date_iso'])
        offset, batch_size = 0, 1000
        while True:
            votes_batch = get_cast_votes(
                hasura_conn, tenant_id, election_event_id, election_id, area_id_value, batch_size, offset)
            if not votes_batch: break
            
            batch_dump_count = 0
            for vote_data in votes_batch:
                vote_writer.writerow([vote_data['voter_id_string'], vote_data['voted_date'].isoformat()])
                batch_dump_count +=1
            total_votes_dumped += batch_dump_count
            logger.info(f"Dumped {batch_dump_count} cast votes (batch) for area {area_id_value}. Total dumped for this area so far: {total_votes_dumped}")

            if len(votes_batch) < batch_size: break # No more full batches means we are done
            offset += len(votes_batch)
    logger.info(f"Finished dumping cast votes for area {area_id_value}. Total votes dumped: {total_votes_dumped}.")
    return total_votes_dumped

# --- Core Processing Function ---
def process_area(area_dict: Dict, tenant_id: str, election_event_id: str,
                keycloak_realm_name: str, main_temp_output_file: str,
                election_id: str,
                hasura_conn, keycloak_conn) -> int:
    area_id = area_dict['id']
    area_name = area_dict.get('name', '')
    logger.info(f"Processing area: {area_name} (ID: {area_id}) using file-based merge.")

    temp_voters_file, temp_votes_file = None, None
    num_voters_for_area = 0
    num_votes_for_area = 0
    written_records_count = 0
    try:
        with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix=f"area_{area_id}_voters_", suffix='.csv') as tmp_f_v:
            temp_voters_file = tmp_f_v.name
        num_voters_for_area = dump_all_voters_to_file(keycloak_conn, keycloak_realm_name, area_id, temp_voters_file)

        with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix=f"area_{area_id}_votes_", suffix='.csv') as tmp_f_c:
            temp_votes_file = tmp_f_c.name
        num_votes_for_area = dump_all_cast_votes_to_file(hasura_conn, tenant_id, election_event_id, election_id, area_id, temp_votes_file)
        
        logger.info(f"Area {area_name} (ID: {area_id}): Dumped {num_voters_for_area} voters and {num_votes_for_area} cast vote records to temporary files.")

        if num_voters_for_area == 0:
            logger.info(f"No voters found for area {area_name} (ID: {area_id}). Skipping merge; 0 records will be written for this area.")
        else:
            with open(main_temp_output_file, 'a', newline='', encoding='utf-8') as main_csv_f, \
                 open(temp_voters_file, 'r', newline='', encoding='utf-8') as voters_f, \
                 open(temp_votes_file, 'r', newline='', encoding='utf-8') as votes_f:

                main_writer = csv.DictWriter(main_csv_f, fieldnames=FINAL_CSV_FIELDNAMES)
                voter_reader, vote_reader = csv.DictReader(voters_f), csv.DictReader(votes_f)
                current_voter, current_vote = next(voter_reader, None), next(vote_reader, None)

                while current_voter:
                    v_id = current_voter['id']
                    voted, vote_date = 'No', ''
                    while current_vote and current_vote['voter_id'] < v_id: current_vote = next(vote_reader, None)
                    if current_vote and current_vote['voter_id'] == v_id:
                        voted, vote_date = 'Yes', current_vote['voted_date_iso']
                        current_vote = next(vote_reader, None)
                    
                    main_writer.writerow({
                        'Voter ID': v_id, 'Username': current_voter.get('username', ''),
                        'First Name': current_voter.get('first_name', ''), 'Middle Name': current_voter.get('middle_name', ''),
                        'Last Name': current_voter.get('last_name', ''), 'Suffix': current_voter.get('suffix', ''),
                        'Area': area_name, 'Registered': current_voter.get('registered_str', 'No'),
                        'Voted': voted, 'Vote Date': vote_date})
                    written_records_count += 1
                    current_voter = next(voter_reader, None)
    finally:
        for f_path in [temp_voters_file, temp_votes_file]:
            if f_path and os.path.exists(f_path): os.remove(f_path); logger.debug(f"Removed temp file: {f_path}")
    logger.info(f"Completed processing area (file-based): {area_name} (ID: {area_id}). Wrote {written_records_count} records to combined report.")
    return written_records_count

# --- Final Output Sorting Function (Using System `sort`) ---
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
                logger.warning(f"Input file '{input_filename}' is empty. Output will be empty.")
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
                    logger.error(f"Sort key '{key_name}' not found in header: {actual_header_cols}. Outputting unsorted.")
                    shutil.copy(input_filename, output_filename)
                    # Count lines in unsorted copied file
                    copied_lines = 0
                    with open(input_filename, 'r', newline='', encoding='utf-8') as src_f:
                        next(src_f, None) # skip header
                        for _ in src_f: copied_lines +=1
                    return copied_lines
            
            if not sort_key_indices:
                logger.error(f"No valid sort keys derived. Outputting unsorted.")
                shutil.copy(input_filename, output_filename)
                copied_lines = 0
                with open(input_filename, 'r', newline='', encoding='utf-8') as src_f:
                    next(src_f, None) # skip header
                    for _ in src_f: copied_lines +=1
                return copied_lines

            with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix="data_to_sort_", suffix=".csv", newline='', encoding='utf-8') as data_temp_file:
                data_temp_path = data_temp_file.name
                line = infile.readline()
                while line:
                    data_temp_file.write(line)
                    data_lines_count += 1 
                    line = infile.readline()
        
        logger.info(f"Extracted {data_lines_count} data lines from '{input_filename}' to '{data_temp_path}' for sorting.")
        
        if data_lines_count == 0: 
            logger.info("Input file contained only a header. Writing header to output.")
            with open(output_filename, 'w', newline='', encoding='utf-8') as outfile:
                outfile.write(header_line + '\n')
            return 0

        with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix="sorted_data_", suffix=".csv", newline='', encoding='utf-8') as sorted_data_file:
            sorted_data_temp_path = sorted_data_file.name

        sort_command = ['sort', '-t', ',']
        sort_command.extend(sort_key_indices)
        sort_command.extend([data_temp_path, '-o', sorted_data_temp_path])

        logger.debug(f"Executing sort command: {' '.join(sort_command)}")
        result = subprocess.run(sort_command, capture_output=True, text=True)

        if result.returncode != 0:
            logger.error(f"System sort command failed. Stderr: {result.stderr}. Stdout: {result.stdout}")
            logger.error(f"Outputting {data_lines_count} unsorted data lines as a fallback.")
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
        open(output_filename, 'w').close()
        return 0
    except Exception as e:
        logger.error(f"An error occurred during system sort: {e}", exc_info=True)
        try:
            if os.path.exists(input_filename):
                shutil.copy(input_filename, output_filename)
                # data_lines_count would be from the attempt to read input_filename
                logger.info(f"Fallback: Copied {data_lines_count if data_lines_count > 0 else 'unsorted'} data to '{output_filename}' due to sort error.")
                return data_lines_count # Returns lines read before error, or 0 if error was early
        except Exception as copy_e:
            logger.error(f"Fallback copy also failed: {copy_e}")
        return 0 
    finally:
        for f_path in [data_temp_path, sorted_data_temp_path]:
            if f_path and os.path.exists(f_path):
                try:
                    os.remove(f_path)
                    logger.debug(f"Removed sort temp file: {f_path}")
                except Exception as e_remove:
                    logger.warning(f"Could not remove sort temp file {f_path}: {e_remove}")

# --- Main Execution ---
def main():
    parser = argparse.ArgumentParser(description='Generate list of voters and their registration/voting status, sorted by name.')
    parser.add_argument('election_alias_pattern', help='Pattern for ILIKE matching election alias.')
    parser.add_argument('--output', default='voter_report_sorted.csv', help='Output CSV file name.')
    args = parser.parse_args()

    temp_unsorted_report_file = None
    hasura_conn, keycloak_conn = None, None 
    total_records_written_to_unsorted_file = 0
    
    try:
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='_combined_unsorted.csv', newline='', encoding='utf-8') as tmp_f:
            temp_unsorted_report_file = tmp_f.name
            csv.DictWriter(tmp_f, fieldnames=FINAL_CSV_FIELDNAMES).writeheader()

        logger.info("Connecting to databases...")
        hasura_conn = connect_to_db(HASURA_DB_CONFIG)
        keycloak_conn = connect_to_db(KEYCLOAK_DB_CONFIG)

        found_election = get_election_by_alias_pattern(
            hasura_conn, TENANT_ID, ELECTION_EVENT_ID_CONST, args.election_alias_pattern
        )

        if not found_election:
            logger.error(f"No election found matching alias pattern '{args.election_alias_pattern}' for tenant '{TENANT_ID}' and event '{ELECTION_EVENT_ID_CONST}'. Process aborted. 0 elections processed.")
            if temp_unsorted_report_file and os.path.exists(temp_unsorted_report_file):
                os.remove(temp_unsorted_report_file)
            sys.exit(1)
        
        logger.info(f"Found 1 election to process.")
        found_election_id = found_election['id']
        found_election_alias = found_election['alias']
        found_election_name = found_election.get('name', 'N/A')
        
        print(f"Processing Election: ID='{found_election_id}', Alias='{found_election_alias}', Name='{found_election_name}'")
        logger.info(f"Processing Election: ID='{found_election_id}', Alias='{found_election_alias}', Name='{found_election_name}'")

        areas = get_areas_by_election_id(hasura_conn, TENANT_ID, ELECTION_EVENT_ID_CONST, found_election_id)
        logger.info(f"Found {len(areas)} areas for this election.")
        
        if not areas:
            logger.warning(f"No areas found for election ID: {found_election_id}. Report will contain only header.")
        else:
            area_processing_summary_logs = []
            for i, area_data in enumerate(areas):
                logger.info(f"--- Processing Area {i+1}/{len(areas)}: {area_data.get('name', 'N/A')} (ID: {area_data.get('id')}) ---")
                records_from_area = process_area(
                    area_data, TENANT_ID, ELECTION_EVENT_ID_CONST,
                    REALM_NAME, temp_unsorted_report_file,
                    found_election_id,
                    hasura_conn, keycloak_conn
                )
                total_records_written_to_unsorted_file += records_from_area
                area_processing_summary_logs.append(f"Area '{area_data.get('name', 'N/A')}': {records_from_area} records written to combined report.")
            
            if area_processing_summary_logs: # Log summary if there was any area processing
                logger.info("--- Area Processing Summary ---")
                for summary_line in area_processing_summary_logs:
                    logger.info(summary_line)
        
        logger.info(f"All areas processed. A total of {total_records_written_to_unsorted_file} records written to unsorted file '{temp_unsorted_report_file}'.")
        
        logger.info("Now sorting the combined report by name using system sort...")
        sorted_data_lines_count = sort_csv_with_system_sort(temp_unsorted_report_file, args.output, FINAL_CSV_FIELDNAMES, SYSTEM_SORT_PRIMARY_KEYS)
        
        # sorted_data_lines_count will be >= 0. -1 isn't a typical return for line counts.
        logger.info(f"Successfully generated sorted report: {args.output} with {sorted_data_lines_count} data records.")

    except Exception as e: 
        logger.error(f"A critical error occurred: {e}", exc_info=True)
        if temp_unsorted_report_file and os.path.exists(temp_unsorted_report_file) and args.output:
             if not (os.path.exists(args.output) and os.path.getsize(args.output) > 0) : 
                try:
                    shutil.copy(temp_unsorted_report_file, args.output)
                    logger.info(f"Fallback: Copied unsorted data (approx. {total_records_written_to_unsorted_file} records) to {args.output} due to critical error.")
                except Exception as copy_e:
                    logger.error(f"Fallback copy also failed: {copy_e}")
        sys.exit(1)
    finally:
        if hasura_conn: hasura_conn.close()
        if keycloak_conn: keycloak_conn.close()
        logger.info("Database connections closed.")
        if temp_unsorted_report_file and os.path.exists(temp_unsorted_report_file):
            os.remove(temp_unsorted_report_file)
            logger.info(f"Removed temporary unsorted report file: {temp_unsorted_report_file}")

if __name__ == '__main__':
    main()
