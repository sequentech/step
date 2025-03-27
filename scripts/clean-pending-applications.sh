#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Script to clean pending applications for accepted applicants


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

# Check database connection
echo "Checking database connection..."
if ! psql -c "\q" "postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME" &>/dev/null; then
  echo "Error: Could not connect to the database. Please check your database credentials and connection."
  exit 1
else
  echo "Database connection successful."
fi

# Create a temporary file for the results
echo "Fetching pending applications data..."
psql -t "postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME" \
  -c "COPY (
    SELECT DISTINCT ON (B.id) 
      B.id,
      B.created_at,
      B.status,
      B.applicant_data->>'email' as email,
      B.applicant_data->>'sequent.read-only.mobile-number' as phone,
      B.applicant_data->>'sequent.read-only.id-card-number' as id_card_number
    FROM sequent_backend.applications A
    JOIN sequent_backend.applications B
    ON (
        A.applicant_data -> 'sequent.read-only.id-card-type' = B.applicant_data -> 'sequent.read-only.id-card-type' AND
        A.applicant_data -> 'sequent.read-only.id-card-number' = B.applicant_data -> 'sequent.read-only.id-card-number'
    ) OR (
        A.applicant_data -> 'emailAndOrMobile' = B.applicant_data -> 'emailAndOrMobile' AND
        A.applicant_data -> 'email' = B.applicant_data -> 'email'
    ) OR (
        A.applicant_data -> 'emailAndOrMobile' = B.applicant_data -> 'emailAndOrMobile' AND
        A.applicant_data -> 'sequent.read-only.mobile-number' = B.applicant_data -> 'sequent.read-only.mobile-number'
    )
    WHERE
        A.status = 'ACCEPTED'
        AND B.status = 'PENDING'
  ) TO STDOUT WITH CSV HEADER;" > applications.csv

# Get the applications count (subtract 1 for header)
PENDING_APPLICATIONS_COUNT=$(( $(wc -l < applications.csv) - 1 ))

if [ "$PENDING_APPLICATIONS_COUNT" -eq 0 ]; then
  echo "No pending applications found that match accepted applications."
  exit 0
fi

# Get the list of application IDs from the CSV (skip header, get first column)
PENDING_APPLICATIONS=$(tail -n +2 applications.csv | cut -d',' -f1 | tr '\n' ',' | sed 's/,$//')

echo "Found $PENDING_APPLICATIONS_COUNT pending applications."
echo "The applications have been saved to applications.csv"
echo -n "Do you want to delete these applications? (yes/no): "
read -r confirmation

if [ "$confirmation" = "yes" ]; then
  echo "Deleting applications..."
  psql "postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME" \
    -c "DELETE FROM sequent_backend.applications WHERE id IN ($PENDING_APPLICATIONS);"
  echo "Applications deleted successfully."
else
  echo "Operation cancelled. No applications were deleted."
fi