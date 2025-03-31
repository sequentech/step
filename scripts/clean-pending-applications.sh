#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Script to clean pending applications for accepted applicants

# Function to display usage information
# Function to display usage information
usage() {
  local exit_code=1
  if [ "$1" = "help" ]; then
    exit_code=0
  fi
  
  echo "Usage: $(basename "$0") OPTION"
  echo "Clean pending applications for accepted applicants."
  echo ""
  echo "Options:"
  echo "  filter-id      Use only ID card number for matching"
  echo "  filter-phone   Use only phone number for matching"
  echo "  filter-email   Use only email for matching" 
  echo "  filter-all     Use all matching criteria"
  echo "  -h, --help     Display this help message"
  echo ""
  exit $exit_code
}

# Check for arguments
if [ $# -eq 0 ]; then
  echo "ERROR: No arguments provided" >&2
  usage
fi

# Define the filter mode based on command line argument
FILTER_MODE=""
if [ "$1" = "filter-id" ] || [ "$1" = "filter-phone" ] || [ "$1" = "filter-email" ] || [ "$1" = "filter-all" ]; then
  FILTER_MODE="$1"
elif [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
  usage "help"
else
  echo "ERROR: Unknown argument: $1" >&2
  usage
fi

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

# Set the JOIN condition based on the filter mode
JOIN_CONDITION=""
case "$FILTER_MODE" in
  "filter-id")
    echo "INFO: Using ID card number filter only"
    JOIN_CONDITION="A.applicant_data -> 'sequent.read-only.id-card-number' = B.applicant_data -> 'sequent.read-only.id-card-number'"
    ;;
  "filter-email")
    echo "INFO: Using email filter only"
    JOIN_CONDITION="A.applicant_data -> 'emailAndOrMobile' = B.applicant_data -> 'emailAndOrMobile' AND A.applicant_data -> 'email' = B.applicant_data -> 'email'"
    ;;
  "filter-phone")
    echo "INFO: Using phone number filter only"
    JOIN_CONDITION="A.applicant_data -> 'emailAndOrMobile' = B.applicant_data -> 'emailAndOrMobile' AND A.applicant_data -> 'sequent.read-only.mobile-number' = B.applicant_data -> 'sequent.read-only.mobile-number'"
    ;;
  *)
    echo "INFO: Using all matching criteria"
    JOIN_CONDITION="(A.applicant_data -> 'sequent.read-only.id-card-number' = B.applicant_data -> 'sequent.read-only.id-card-number') OR (A.applicant_data -> 'emailAndOrMobile' = B.applicant_data -> 'emailAndOrMobile' AND A.applicant_data -> 'email' = B.applicant_data -> 'email') OR (A.applicant_data -> 'emailAndOrMobile' = B.applicant_data -> 'emailAndOrMobile' AND A.applicant_data -> 'sequent.read-only.mobile-number' = B.applicant_data -> 'sequent.read-only.mobile-number')"
    ;;
esac

# Fetching data
echo "INFO: Fetching pending applications data..."
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
    ON 
        $JOIN_CONDITION
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

# Get the list of application IDs from the CSV, properly quoted for SQL
# Use awk to add single quotes around each UUID and separate with commas
PENDING_APPLICATIONS=$(tail -n +2 applications.csv | cut -d',' -f1 | awk '{print "'"'"'" $0 "'"'"'"}' | tr '\n' ',' | sed 's/,$//')

echo "Found $PENDING_APPLICATIONS_COUNT pending applications."
echo "The applications have been saved to applications.csv"
echo -n "Do you want to reject these applications? (yes/no): "
read -r confirmation

if [ "$confirmation" = "yes" ]; then
  echo "Rejecting applications..."
  psql "postgresql://$HASURA_DB__USER:$HASURA_DB__PASSWORD@$HASURA_DB__HOST:$HASURA_DB__PORT/$HASURA_DB__DBNAME" \
    -c "UPDATE
          sequent_backend.applications
        SET
          status = 'REJECTED'
        WHERE
          id IN ($PENDING_APPLICATIONS)
          AND status = 'PENDING';"
  echo "Applications rejected successfully."
else
  echo "Operation cancelled. No applications were rejected."
fi