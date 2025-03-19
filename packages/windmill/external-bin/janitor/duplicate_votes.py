import json
import psycopg2
from psycopg2.extras import execute_values
import os
from datetime import datetime
import csv
import io


with open('duplicate_votes_data.json', 'r') as config_file:
    config = json.load(config_file)

realm_name = config.get("realm_name")
target_row_count = config.get("target_row_count", 100)
row_id_to_clone = config.get("row_id_to_clone")

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

print("number of rows to clone: ", target_row_count)

kc_cursor = keycloak_conn.cursor()
hasura_cursor = hasura_conn.cursor()

#Offset should start at 0 and can be changed if you want to add more votes


hasura_cursor.execute(
    """
    SELECT election_id, tenant_id, area_id, annotations, content, cast_ballot_signature, election_event_id, ballot_id
        FROM sequent_backend.cast_vote WHERE id = %s""", (row_id_to_clone,))
base_row = hasura_cursor.fetchone()

if not base_row:
    print("No row found to clone.")
    raise Exception("Something went wrong! no base row")
else:
    election_id, tenant_id, area_id,annotations, content, cast_ballot_signature, election_event_id, ballot_id = base_row
    annotations_json = json.dumps(annotations)

# get_user_ids_query = """
# SELECT ue.id, ue.username, r.name AS realm_name
# FROM user_entity AS ue
# JOIN realm AS r ON ue.realm_id = r.id
# WHERE r.name = %s AND ue.area-id = %s
# LIMIT %s
# OFFSET 0; 
# """

get_user_ids_query = """SELECT ue.id, ue.username, r.name AS realm_name
FROM user_entity AS ue
JOIN realm AS r ON ue.realm_id = r.id
JOIN user_attribute AS ua ON ue.id = ua.user_id
WHERE r.name = %s
  AND ua.name = %s
  AND ua.value = %s
LIMIT %s
OFFSET 0;
"""

print("area_id", area_id)
kc_cursor.execute(get_user_ids_query, (realm_name,"area-id", area_id, target_row_count))
existing_user_ids = [row[0] for row in kc_cursor.fetchall()]
print("number of existing user ids: ", len(existing_user_ids))



insert_query = """
    INSERT INTO sequent_backend.cast_vote (
        voter_id_string, election_id, tenant_id, area_id,annotations, content,
        cast_ballot_signature, election_event_id, ballot_id
    )
    VALUES %s
"""
print("existing_user_ids", len(existing_user_ids))
batch_size = 100
rows_to_insert = []
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


hasura_cursor.copy_expert(copy_sql, output)
hasura_conn.commit()



    # start_time = datetime.now()
    # print("Start time:", start_time)

    # for i in range(0, len(rows_to_insert), batch_size):
    #     print(f"batch number ${i} started")
    #     batch = rows_to_insert[i:i+batch_size]
    #     execute_values(hasura_cursor, insert_query, batch, template=None, page_size=batch_size)
    #     hasura_conn.commit()  # Commit after each batch
    #     print(f"batch number ${i} finished")

    # end_time = datetime.now()
    # print("End time:", end_time)

# Cleanup
kc_cursor.close()
keycloak_conn.close()
hasura_cursor.close()
hasura_conn.close()