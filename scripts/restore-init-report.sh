#!/bin/bash

# Script to restore a init report or tally to in progress

# Check if tally session ID is provided
if [ -z "$1" ]; then
  echo "Error: Tally session ID is required"
  echo "Usage: $0 <tally_session_id>"
  exit 1
fi

# Set tally session ID from first argument
TALLY_SESSION_ID="$1"

# Check if script is run as root
if [ "$EUID" -ne 0 ]; then
  echo "This script must be run as root"
  exit 1
fi

# Check if required environment variables are set
if [ -z "$HASURA_DB__USER" ] || [ -z "$HASURA_DB__PASSWORD" ] || [ -z "$HASURA_DB__HOST" ] || [ -z "$HASURA_DB__PORT" ] || [ -z "$HASURA_DB__DBNAME" ]; then
  echo "Error: One or more required database environment variables are not set"
  echo "Required variables: HASURA_DB__USER, HASURA_DB__PASSWORD, HASURA_DB__HOST, HASURA_DB__PORT, HASURA_DB__DBNAME"
  exit 1
fi

# Check database connection
echo "Checking database connection..."
if ! psql -c "\q" "postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME" &>/dev/null; then
  echo "Error: Could not connect to the database. Please check your database credentials and connection."
  exit 1
else
  echo "Database connection successful."
fi

# Check if postgresql-client is already installed
echo "Checking if postgresql-client is installed..."
if dpkg -l | grep -q postgresql-client; then
  echo "postgresql-client is already installed."
else
  echo "postgresql-client is not installed. Installing now..."
  
  # Update package lists
  echo "Updating package lists..."
  if apt update -y; then
    echo "Package lists updated successfully."
  else
    echo "Failed to update package lists. Exiting."
    exit 1
  fi
  
  # Install postgresql-client
  echo "Installing postgresql-client..."
  if apt install -y postgresql-client; then
    echo "postgresql-client installed successfully."
  else
    echo "Failed to install postgresql-client. Exiting."
    exit 1
  fi
fi

echo "Querying database for election alias..."

# Check if initialization report exists
echo "Checking if initialization report exists..."
INIT_REPORT_COUNT=$(psql -t postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME \
  -c "SELECT COUNT(*)
      FROM sequent_backend.tally_session
      WHERE
      id = '$TALLY_SESSION_ID'
      AND tally_type = 'INITIALIZATION_REPORT'
      AND is_execution_completed IS TRUE
      AND execution_status != 'IN_PROGRESS';" 2>/dev/null | tr -d '[:space:]')

if [ "$INIT_REPORT_COUNT" -eq "0" ]; then
  echo "Error: No valid initialization report found for tally session ID: $TALLY_SESSION_ID"
  exit 1
fi

# Run the query to get the election alias
ELECTION_ALIAS=$(psql -t postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME \
  -c "SELECT election.alias
      FROM sequent_backend.tally_session
      JOIN sequent_backend.election
      ON election.id = ANY(tally_session.election_ids)
      WHERE tally_session.id = '$TALLY_SESSION_ID';" 2>/dev/null | tr -d '[:space:]')

if [ -z "$ELECTION_ALIAS" ]; then
  echo "Error: Could not find election for tally session ID: $TALLY_SESSION_ID"
  exit 1
fi

# Ask for confirmation
echo "Are you sure you want to restore the initialization report for election $ELECTION_ALIAS? write yes: "
read -r CONFIRMATION

if [ "$CONFIRMATION" != "yes" ]; then
  echo "Operation cancelled."
  exit 0
fi


# Execute transaction to update tally session status and delete execution
echo "Executing database transaction..."
TRANSACTION_RESULT=$(psql -t postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME \
  -c "BEGIN;
      -- Delete the most recent tally session execution
      DELETE FROM sequent_backend.tally_session_execution 
      WHERE id = (
        SELECT id 
        FROM sequent_backend.tally_session_execution 
        WHERE tally_session_id = '$TALLY_SESSION_ID' 
        ORDER BY created_at DESC 
        LIMIT 1
      );
      
      -- Update tally session status to IN_PROGRESS
      UPDATE sequent_backend.tally_session
      SET is_execution_completed = FALSE,
          execution_status = 'IN_PROGRESS'
      WHERE id = '$TALLY_SESSION_ID';
      
      COMMIT;
      
      -- Return affected row to confirm
      SELECT id FROM sequent_backend.tally_session WHERE id = '$TALLY_SESSION_ID';" 2>/dev/null | tr -d '[:space:]')

if [ -z "$TRANSACTION_RESULT" ]; then
  echo "Error: Transaction failed. No changes were made to the database."
  exit 1
else
  echo "Transaction successful. Initialization report $TALLY_SESSION_ID has been reset to IN_PROGRESS state."
fi

echo "Proceeding with the operation..."
echo "Done."