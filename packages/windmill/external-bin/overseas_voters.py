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
from concurrent.futures import ThreadPoolExecutor
from typing import List, Dict, Optional, Tuple, Set

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Database connection information will come from environment variables
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

# Constants from the Rust code
AREA_ID_ATTR_NAME = "area_id"
VALIDATE_ID_ATTR_NAME = "validate_id"
VALIDATE_ID_REGISTERED_VOTER = "registered-voter"

def connect_to_db(config: Dict[str, str]):
    """Establish a database connection with the given configuration."""
    try:
        conn = psycopg2.connect(**config)
        return conn
    except Exception as e:
        logger.error(f"Database connection error: {e}")
        raise

def get_realm_id(conn, realm_name: str) -> str:
    """Get the realm ID from Keycloak database."""
    with conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT id::VARCHAR AS id
            FROM realm
            WHERE realm.name = %s
        """, (realm_name,))
        
        rows = cursor.fetchall()
        if not rows:
            raise ValueError(f"Realm not found: {realm_name}")
        if len(rows) > 1:
            raise ValueError(f"Found too many realms with same name: {len(rows)}")
        
        return rows[0]['id']

def get_areas_by_election_id(hasura_conn, tenant_id: str, election_event_id: str, election_id: str) -> List[Dict]:
    """Get areas related to a specific election."""
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT DISTINCT ON (a.id)
                a.*
            FROM
                sequent_backend.area a
            JOIN
                sequent_backend.area_contest ac ON
                    a.id = ac.area_id AND
                    a.election_event_id = ac.election_event_id AND
                    a.tenant_id = ac.tenant_id
            JOIN
                sequent_backend.contest c ON
                    ac.contest_id = c.id AND
                    ac.election_event_id = c.election_event_id AND
                    ac.tenant_id = c.tenant_id
            WHERE
                c.tenant_id = %s AND
                c.election_event_id = %s AND
                c.election_id = %s
        """, (
            tenant_id,
            election_event_id,
            election_id,
        ))
        
        return cursor.fetchall()

def get_election_by_id(hasura_conn, tenant_id: str, election_event_id: str, election_id: str) -> Optional[Dict]:
    """Get election details by ID."""
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT
                *
            FROM
                sequent_backend.election
            WHERE
                tenant_id = %s AND
                election_event_id = %s AND
                id = %s
        """, (
            tenant_id,
            election_event_id,
            election_id,
        ))
        
        rows = cursor.fetchall()
        return rows[0] if rows else None

def get_non_test_elections(hasura_conn, tenant_id: str, election_event_id: str) -> List[Dict]:
    """Get all non-test elections."""
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        cursor.execute("""
            SELECT
                *
            FROM
                sequent_backend.election
            WHERE
                tenant_id = %s AND
                election_event_id = %s AND
                name NOT ILIKE '%Test%' AND
                alias NOT ILIKE '%Test%'
        """, (
            tenant_id,
            election_event_id,
        ))
        
        return cursor.fetchall()

def get_voters_by_area_id(keycloak_conn, realm: str, area_id: str, 
                          batch_size: int = 1000, offset: int = 0) -> Tuple[List[Dict], Optional[int]]:
    """
    Get voters by area ID with pagination.
    Returns a tuple of (voters_list, next_offset)
    """
    with keycloak_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        sql = f"""
        SELECT
            u.id,
            u.first_name,
            u.last_name,
            u.username,
            COALESCE(attr_json.attributes ->> 'middleName', '') AS middle_name,
            COALESCE(attr_json.attributes ->> 'suffix', '') AS suffix,
            COALESCE(attr_json.attributes ->> '{VALIDATE_ID_ATTR_NAME}', '') AS validate_id,
            COUNT(u.id) OVER() AS total_count
        FROM
            user_entity u
        INNER JOIN
            realm AS ra ON ra.id = u.realm_id
        LEFT JOIN LATERAL (
            SELECT
                json_object_agg(ua.name, ua.value) AS attributes
            FROM user_attribute ua
            WHERE ua.user_id = u.id
            GROUP BY ua.user_id
        ) attr_json ON true
        WHERE
            ra.name = %s AND
            EXISTS (
                SELECT 1
                FROM user_attribute ua
                WHERE ua.user_id = u.id
                AND ua.name = '{AREA_ID_ATTR_NAME}'
                AND ua.value = %s
            )
        ORDER BY u.last_name, u.first_name
        LIMIT %s OFFSET %s
        """
        
        cursor.execute(sql, (realm, area_id, batch_size, offset))
        rows = cursor.fetchall()
        
        # Process the results
        voters = []
        for row in rows:
            voter = {
                'id': row['id'],
                'first_name': row['first_name'],
                'last_name': row['last_name'],
                'username': row['username'],
                'middle_name': row['middle_name'],
                'suffix': row['suffix'],
                'status': None if row['validate_id'] == VALIDATE_ID_REGISTERED_VOTER else "Did Not Pre-enroll",
                'date_voted': None,
                'registered': row['validate_id'] == VALIDATE_ID_REGISTERED_VOTER
            }
            voters.append(voter)
        
        # Calculate next offset for pagination
        total_count = int(rows[0]['total_count']) if rows else 0
        next_offset = offset + len(rows) if offset + len(rows) < total_count else None
        
        return voters, next_offset

def get_cast_votes(hasura_conn, tenant_id: str, election_event_id: str, 
                  area_id: Optional[str] = None, election_ids: Optional[List[str]] = None,
                  batch_size: int = 1000, offset: int = 0) -> List[Dict]:
    """
    Get cast votes for specific election(s) and area, filtered to only include 
    non-test elections. Uses streaming through batches.
    """
    election_filter = "AND v.election_id = ANY(%s)" if election_ids else ""
    area_filter = "AND v.area_id = %s" if area_id else ""
    
    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        sql = f"""
        SELECT 
            v.voter_id_string,
            v.election_id,
            MAX(v.created_at) AS voted_date,
            e.name AS election_name,
            e.alias AS election_alias
        FROM 
            sequent_backend.cast_vote v
        JOIN
            sequent_backend.election e ON v.election_id = e.id
        WHERE 
            v.tenant_id = %s AND
            v.election_event_id = %s
            {area_filter}
            {election_filter}
            AND e.name NOT ILIKE '%Test%'
            AND (e.alias IS NULL OR e.alias NOT ILIKE '%Test%')
        GROUP BY 
            v.voter_id_string, 
            v.election_id,
            e.name,
            e.alias
        ORDER BY 
            v.voter_id_string
        LIMIT %s OFFSET %s
        """
        
        params = [tenant_id, election_event_id]
        if area_id:
            params.append(area_id)
        if election_ids:
            params.append(election_ids)
        params.extend([batch_size, offset])
        
        cursor.execute(sql, params)
        return cursor.fetchall()

def process_area(area: Dict, tenant_id: str, election_event_id: str, election_id: str, 
                realm: str, output_file: str, hasura_conn, keycloak_conn) -> None:
    """
    Process a single area - retrieve voters and their voting status,
    then write to CSV using streaming.
    """
    area_id = area['id']
    area_name = area.get('name', '')
    
    logger.info(f"Processing area: {area_name} (ID: {area_id})")
    
    # Get non-test election IDs for this election event
    non_test_elections = get_non_test_elections(hasura_conn, tenant_id, election_event_id)
    non_test_election_ids = [e['id'] for e in non_test_elections]
    
    # Open the output file in append mode to collect all areas
    with open(output_file, 'a', newline='') as csvfile:
        fieldnames = ['Voter ID', 'Username', 'First Name', 'Middle Name', 'Last Name', 
                     'Suffix', 'Area', 'Registered', 'Voted', 'Vote Date']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        
        # Write header only if file is empty
        if csvfile.tell() == 0:
            writer.writeheader()
        
        # Get all cast votes for this area - we'll use in-memory dictionary 
        # since votes are typically far fewer than voters
        vote_dict = {}
        offset = 0
        while True:
            votes_batch = get_cast_votes(
                hasura_conn, 
                tenant_id, 
                election_event_id, 
                area_id, 
                non_test_election_ids,
                batch_size=1000, 
                offset=offset
            )
            
            if not votes_batch:
                break
                
            for vote in votes_batch:
                voter_id = vote['voter_id_string']
                if voter_id not in vote_dict or vote['voted_date'] > vote_dict[voter_id]['voted_date']:
                    vote_dict[voter_id] = vote
                    
            offset += len(votes_batch)
            if len(votes_batch) < 1000:
                break                                                                                                                                                                 
            params.append(election_ids)
        params.extend([batch_size, offset])

        cursor.execute(sql, params)
        return cursor.fetchall()

def process_area(area: Dict, tenant_id: str, election_event_id: str, election_id: str,
                realm: str, output_file: str, hasura_conn, keycloak_conn) -> None:
    """
    Process a single area - retrieve voters and their voting status,
    then write to CSV using streaming.
    """
    area_id = area['id']
    area_name = area.get('name', '')

    logger.info(f"Processing area: {area_name} (ID: {area_id})")

    # Get non-test election IDs for this election event
    non_test_elections = get_non_test_elections(hasura_conn, tenant_id, election_event_id)
    non_test_election_ids = [e['id'] for e in non_test_elections]

    # Open the output file in append mode to collect all areas
    with open(output_file, 'a', newline='') as csvfile:
        fieldnames = ['Voter ID', 'Username', 'First Name', 'Middle Name', 'Last Name',
                     'Suffix', 'Area', 'Registered', 'Voted', 'Vote Date']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)

        # Write header only if file is empty
        if csvfile.tell() == 0:
            writer.writeheader()

        # Get all cast votes for this area - we'll use in-memory dictionary
        # since votes are typically far fewer than voters
        vote_dict = {}
        offset = 0
        while True:
            votes_batch = get_cast_votes(
                hasura_conn,
                tenant_id,
                election_event_id,
                area_id,
                non_test_election_ids,
                batch_size=1000,
                offset=offset
            )

            if not votes_batch:
                break

            for vote in votes_batch:
                voter_id = vote['voter_id_string']
                if voter_id not in vote_dict or vote['voted_date'] > vote_dict[voter_id]['voted_date']:
                    vote_dict[voter_id] = vote

            offset += len(votes_batch)
            if len(votes_batch) < 1000:
                break

def get_cast_votes(
    hasura_conn,
    tenant_id: str,
    election_event_id: str,
    area_id: Optional[str] = None,
    election_ids: Optional[List[str]] = None,
    batch_size: int = 1000,
    offset: int = 0
) -> List[Dict]:
    """
    Get cast votes for specific election(s) and area, filtered to only include
    non-test elections. Uses streaming through batches.
    """
    election_filter = "AND v.election_id = ANY(%s)" if election_ids else ""
    area_filter = "AND v.area_id = %s" if area_id else ""

    with hasura_conn.cursor(cursor_factory=RealDictCursor) as cursor:
        sql = f"""
        SELECT
            v.voter_id_string,
            v.election_id,
            MAX(v.created_at) AS voted_date,
            e. name AS election_name,
            e.alias AS election_alias
        FROM
            sequent_backend.cast_vote v
        JOIN
            sequent_backend. election e ON v.election_id = e. id
        WHERE
            v. tenant_id = %s AND
            v.election_event_id = %s
            {area_filter}
            {election_filter}
            AND e.name NOT ILIKE '%test%'
            AND (e.alias IS NULL OR e.alias NOT ILIKE '%test%' )
        GROUP BY
            v.voter_id_string,
            v.election_id,
            e.name, e.alias
        ORDER BY
            v.voter_id_string
        LIMIT %S OFFSET %s
        """

        params = [tenant_id, election_event_id]
        if area_id:
            params.append(area_id)
        if election_ids:
            params.append(election_ids)
        params.extend([batch_size, offset])

        cursor.execute(sql, params)
        return cursor.fetchall()

def process_area(
    area: Dict, 
    tenant_id: str, 
    election_event_id: str, 
    election_id: str,
    realm: str, 
    output_file: str, 
    hasura_conn, 
    keycloak_conn
) -> None:
    """
    Process a single area - retrieve voters and their voting status,
    then write to CSV using streaming.
    """
    area_id = area['id']
    area_name = area.get('name', '')

    logger.info(f"Processing area: {area_name} (ID: {area_id})")

    # Get non-test election IDs for this election event
    non_test_elections = get_non_test_elections(hasura_conn, tenant_id, election_event_id)
    non_test_election_ids = [e['id'] for e in non_test_elections]

    # Open the output file in append mode to collect all areas
    with open(output_file, 'a', newline='') as csvfile:
        fieldnames = ['Voter ID', 'Username', 'First Name', 'Middle Name', 'Last Name',
                     'Suffix', 'Area', 'Registered', 'Voted', 'Vote Date']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)

        # Write header only if file is empty
        if csvfile.tell() == 0:
            writer.writeheader()

        # Get all cast votes for this area - we'll use in-memory dictionary
        # since votes are typically far fewer than voters
        vote_dict = {}
        offset = 0
        while True:
            votes_batch = get_cast_votes(
                hasura_conn,
                tenant_id,
                election_event_id,
                area_id,
                non_test_election_ids,
                batch_size=1000,
                offset=offset
            )

            if not votes_batch:
                break

            for vote in votes_batch:
                voter_id = vote['voter_id_string']
                if voter_id not in vote_dict or vote['voted_date'] > vote_dict[voter_id]['voted_date']:
                    vote_dict[voter_id] = vote

            offset += len(votes_batch)
            if len(votes_batch) < 1000:
                break

        # Now stream all voters and join with votes information
        offset = 0
        while True:
            voters_batch, next_offset = get_voters_by_area_id(
                keycloak_conn,
                realm,
                area_id,
                batch_size=1000,
                offset=offset
            )
            if not voters_batch:
                break

            # Process each voter and join with voting info
            for voter in voters_batch:
                voter_id = voter['id']
                vote_info = vote_dict.get(voter_id)

                # Write to CSV
                writer.writerow({
                    'Voter ID': voter_id,
                    'Username': voter.get('username', ''),
                    'First Name': voter.get('first_name', ''),
                    'Middle Name': voter.get('middle_name', ''),
                    'Last Name': voter.get('last_name', ''),
                    'Suffix': voter.get('suffix', ''),
                    'Area': area_name,
                    'Registered': 'Yes' if voter.get('registered') else 'No',
                    'Voted': 'Yes' if vote_info else 'No',
                    'Vote Date': vote_info['voted_date'] if vote_info else ''
                })

            if not next_offset:
                break

            offset = next_offset

    logger.info(f"Completed processing area: {area_name}")

def main():
    parser = argparse.ArgumentParser(description='Generate list of overseas voters and their registration/voting status')
    parser.add_argument('election_id', help='The election ID to process')
    parser.add_argument('--output', default='overseas_voters.csv', help='Output CSV file (default: overseas_voters.csv)')
    parser.add_argument('--tenant-id', default=os.environ.get('SUPER_ADMIN_TENANT_ID'), help='Tenant ID')
    parser.add_argument('--election-event-id', help='Election event ID')
    parser.add_argument('--batch-size', type=int, default=1000, help='Batch size for database queries')
    parser.add_argument('--threads', type=int, default=1, help='Number of threads to use for processing')

    args = parser.parse_args()

    # Validate required arguments
    if not args.tenant_id:
        parser.error("Tenant ID must be provided either through --tenant-id or SUPER_ADMIN_TENANT_ID environment variable")

    # Connect to databases
    logger.info("Connecting to databases...")
    hasura_conn = connect_to_db(HASURA_DB_CONFIG)
    keycloak_conn = connect_to_db(KEYCLOAK_DB_CONFIG)

    try:
        # Retrieve election information
        election = get_election_by_id(hasura_conn, args.tenant_id, args.election_event_id, args.election_id)
        if not election:
            logger.error(f"Election not found: {args.election_id}")
            return 1

        logger.info(f"Processing election: {election.get('name', '')} (ID: {args.election_id})")

        # Determine the realm for Keycloak queries
        realm = f"tenant-{args.tenant_id}-event-{args.election_event_id}"
        # Now stream all voters and join with votes information
        offset = 0
        while True:
            voters_batch, next_offset = get_voters_by_area_id(
                keycloak_conn, 
                realm, 
                area_id,
                batch_size=1000, 
                offset=offset
            )
            
            if not voters_batch:
                break
                
            # Process each voter and join with voting info
            for voter in voters_batch:
                voter_id = voter['id']
                vote_info = vote_dict.get(voter_id)
                
                # Write to CSV
                writer.writerow({
                    'Voter ID': voter_id,
                    'Username': voter.get('username', ''),
                    'First Name': voter.get('first_name', ''),
                    'Middle Name': voter.get('middle_name', ''),
                    'Last Name': voter.get('last_name', ''),
                    'Suffix': voter.get('suffix', ''),
                    'Area': area_name,
                    'Registered': 'Yes' if voter.get('registered') else 'No',
                    'Voted': 'Yes' if vote_info else 'No',
                    'Vote Date': vote_info['voted_date'].isoformat() if vote_info and vote_info.get('voted_date') else ''
                })
            
            if not next_offset:
                break
                
            offset = next_offset
    
    logger.info(f"Completed processing area: {area_name}")

def main():
    parser = argparse.ArgumentParser(description='Generate list of overseas voters and their registration/voting status')
    parser.add_argument('election_id', help='The election ID to process')
    parser.add_argument('--output', default='overseas_voters.csv', help='Output CSV file (default: overseas_voters.csv)')
    parser.add_argument('--tenant-id', required=True, help='Tenant ID (e.g., from SUPER_ADMIN_TENANT_ID env var)')
    parser.add_argument('--election-event-id', required=True, help='Election event ID')
    # Removed --threads as we are doing single-threaded processing
    # Batch size is used internally by functions, not a direct user arg for threading control
    
    args = parser.parse_args()
    
    # Clear the output file if it exists before starting
    if os.path.exists(args.output):
        os.remove(args.output)
        logger.info(f"Removed existing output file: {args.output}")

    # Connect to databases
    logger.info("Connecting to databases...")
    hasura_conn = connect_to_db(HASURA_DB_CONFIG)
    keycloak_conn = connect_to_db(KEYCLOAK_DB_CONFIG)
    
    try:
        # Retrieve election information (to confirm it exists, primarily)
        election = get_election_by_id(hasura_conn, args.tenant_id, args.election_event_id, args.election_id)
        if not election:
            logger.error(f"Election not found: {args.election_id} for tenant {args.tenant_id} and event {args.election_event_id}")
            sys.exit(1)
        
        logger.info(f"Processing election: {election.get('name', '')} (ID: {args.election_id})")
        
        # Determine the realm for Keycloak queries
        # The Rust code uses: get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);
        # which constructs a string like: "tenant-{tenant_id}-event-{election_event_id}"
        realm = f"tenant-{args.tenant_id}-event-{args.election_event_id}"
        
        # Get areas for the specified election
        areas = get_areas_by_election_id(hasura_conn, args.tenant_id, args.election_event_id, args.election_id)
        if not areas:
            logger.warning(f"No areas found for election ID: {args.election_id}")
            sys.exit(0) # Not an error, just no data
            
        logger.info(f"Found {len(areas)} areas for election {args.election_id}. Processing sequentially...")

        for area in areas:
            process_area(
                area,
                args.tenant_id,
                args.election_event_id,
                args.election_id,
                realm,
                args.output, # Pass the main output file
                hasura_conn,
                keycloak_conn
            )
            
        logger.info(f"Successfully generated report: {args.output}")

    except Exception as e:
        logger.error(f"An error occurred: {e}", exc_info=True)
        sys.exit(1)
    finally:
        if 'hasura_conn' in locals() and hasura_conn:
            hasura_conn.close()
        if 'keycloak_conn' in locals() and keycloak_conn:
            keycloak_conn.close()
        logger.info("Database connections closed.")

if __name__ == '__main__':
    main()
