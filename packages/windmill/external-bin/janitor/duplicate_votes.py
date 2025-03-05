import json
import psycopg2
from psycopg2.extras import execute_values

# --- Load configuration from JSON file ---
with open('duplicate_votes_data.json', 'r') as config_file:
    config = json.load(config_file)

# Extract configuration values
realm_name = config.get("realm_name")
target_row_count = config.get("target_row_count", 100)  # default to 100 if not provided
row_id_to_clone = config.get("row_id_to_clone")

# Connections
keycloak_conn = psycopg2.connect(
    dbname="postgres",        # <--- The name of your database
    user="postgres",          # <--- The database user to authenticate as
    password="postgrespassword",          # <--- The user’s password
    host="postgres-keycloak", # <--- The hostname or IP
    port=5432                 # <--- Port (if different, set your custom port)
)
hasura_conn = psycopg2.connect(
    dbname="postgres",        # <--- The name of your database
    user="postgres",          # <--- The database user to authenticate as
    password="postgrespassword",          # <--- The user’s password
    host="postgres", # <--- The hostname or IP
    port=5432                 # <--- Port (if different, set your custom port)
)

print("number of rows to clone: ", target_row_count)

kc_cursor = keycloak_conn.cursor()
hasura_cursor = hasura_conn.cursor()


get_user_ids_query = """
SELECT ue.id, ue.username, r.name AS realm_name
FROM user_entity AS ue
JOIN realm AS r ON ue.realm_id = r.id
WHERE r.name = %s
LIMIT %s
OFFSET 0;
"""

kc_cursor.execute(get_user_ids_query, (realm_name, target_row_count))
existing_user_ids = [row[0] for row in kc_cursor.fetchall()]
print("number of existing user ids: ", len(existing_user_ids))

hasura_cursor.execute(
    """
    SELECT election_id, tenant_id, area_id, content, cast_ballot_signature, election_event_id, ballot_id
        FROM sequent_backend.cast_vote WHERE id = %s""", (row_id_to_clone,))
base_row = hasura_cursor.fetchone()

if not base_row:
    print("No row found to clone.")
else:
    election_id, tenant_id, area_id, content, cast_ballot_signature, election_event_id, ballot_id = base_row
    # annotations_json = json.dumps(annotations)

    insert_query = """
    INSERT INTO sequent_backend.cast_vote (
        voter_id_string, election_id, tenant_id, area_id, content,
        cast_ballot_signature, election_event_id, ballot_id
    )
    VALUES %s
"""

    batch_size = 1000
    rows_to_insert = []
    for i in range(len(existing_user_ids)):
        uid = existing_user_ids[i]
        rows_to_insert.append((
            uid, election_id, tenant_id, area_id, content,
            cast_ballot_signature, election_event_id, ballot_id
        ))
        print("rows_to_insert", i)
    
        
    # print("rows_to_insert", len(rows_to_insert))
    # for i in range(0, len(rows_to_insert), batch_size):
    #     batch = rows_to_insert[i:i+batch_size]
    #     execute_values(hasura_cursor, insert_query, batch, template=None, page_size=batch_size)
    #     hasura_conn.commit()  # Commit after each batch

    execute_values(hasura_cursor, insert_query, rows_to_insert, template=None, page_size=1000)
    
    hasura_conn.commit()

# Cleanup
kc_cursor.close()
keycloak_conn.close()
hasura_cursor.close()
hasura_conn.close()