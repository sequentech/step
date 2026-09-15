#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Script to gather monitoring metrics for a specific Election ID within an Election Event / Realm

# Check if script is run as root
if [ "$EUID" -ne 0 ]; then
  echo "Error: This script must be run as root"
  exit 1
fi

DEFAULT_REALM_NAME="tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-508acaa6-fdac-4e67-a03d-f25ed36cde8d"
REALM_NAME="${2:-$DEFAULT_REALM_NAME}"
ELECTION_PATTERN="${1}"

if [ -z "$ELECTION_PATTERN" ]; then
  echo "Error: ELECTION_PATTERN is a mandatory first argument."
  echo "Usage: $0 ELECTION_PATTERN [REALM_NAME]"
  exit 1
fi

TENANT_ID="${REALM_NAME##tenant-}"
TENANT_ID="${TENANT_ID%%-event-*}"
ELECTION_EVENT_ID="${REALM_NAME##*-event-}"

echo "REALM_NAME: ${REALM_NAME}"
echo "TENANT_ID: ${TENANT_ID}"
echo "ELECTION_EVENT_ID: ${ELECTION_EVENT_ID}"
echo "ELECTION_PATTERN: ${ELECTION_PATTERN}"


# Check if required environment variables are set
REQUIRED_VARS=(
    "HASURA_DB__USER" "HASURA_DB__PASSWORD" "HASURA_DB__HOST" "HASURA_DB__PORT" "HASURA_DB__DBNAME"
    "KEYCLOAK_DB__USER" "KEYCLOAK_DB__PASSWORD" "KEYCLOAK_DB__HOST" "KEYCLOAK_DB__PORT" "KEYCLOAK_DB__DBNAME"
)
SHOULD_EXIT=false
for VAR_NAME in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!VAR_NAME}" ]; then
        echo "Error: Required environment variable ${VAR_NAME} is not set."
        SHOULD_EXIT=true
    fi
done
if [ "$SHOULD_EXIT" = true ]; then
    echo "Please set all required database environment variables."
    exit 1
fi

# Check and install dependencies
install_if_missing() {
  PKG_NAME=$1
  echo "Checking if ${PKG_NAME} is installed..."
  if dpkg -l | grep -q "^ii\s*${PKG_NAME}\s"; then
    echo "${PKG_NAME} is already installed."
  else
    echo "${PKG_NAME} is not installed. Installing now..."
    if ! apt update -y; then echo "Failed to update package lists. Exiting."; exit 1; fi
    if ! apt install -y "${PKG_NAME}"; then echo "Failed to install ${PKG_NAME}. Exiting."; exit 1; fi
    echo "${PKG_NAME} installed successfully."
  fi
}

install_if_missing "postgresql-client"
install_if_missing "jq" # Though not used in this specific script version, good to keep for consistency

# Database connection checks
check_db_connection() {
  DB_TYPE=$1
  DB_HOST_VAR="${DB_TYPE}_DB__HOST"
  DB_USER_VAR="${DB_TYPE}_DB__USER"
  DB_PASS_VAR="${DB_TYPE}_DB__PASSWORD"
  DB_PORT_VAR="${DB_TYPE}_DB__PORT"
  DB_NAME_VAR="${DB_TYPE}_DB__DBNAME"

  echo -e "\nChecking ${DB_TYPE} database connection..."
  export PGPASSWORD="${!DB_PASS_VAR}"
  if ! psql -h "${!DB_HOST_VAR}" \
      -U "${!DB_USER_VAR}" \
      -p "${!DB_PORT_VAR}" \
      -d "${!DB_NAME_VAR}" \
      -c "\q" &>/dev/null;
  then
    echo "Error: Could not connect to the ${DB_TYPE} database. Please check your database credentials and connection."
    unset PGPASSWORD
    exit 1
  else
    echo "${DB_TYPE} Database connection successful."
  fi
  unset PGPASSWORD
}

# Helper function to run psql commands against Hasura DB
run_hasura_psql() {
  export PGPASSWORD="$HASURA_DB__PASSWORD"
  psql -h "$HASURA_DB__HOST" \
    -U "$HASURA_DB__USER" \
    -p "$HASURA_DB__PORT" \
    -d "$HASURA_DB__DBNAME" \
    -t -A -c "$1"
  local exit_code=$?
  unset PGPASSWORD
  return $exit_code
}

# Helper function to run psql commands against Keycloak DB
run_keycloak_psql() {
  export PGPASSWORD="$KEYCLOAK_DB__PASSWORD"
  psql -h "$KEYCLOAK_DB__HOST" \
    -U "$KEYCLOAK_DB__USER" \
    -p "$KEYCLOAK_DB__PORT" \
    -d "$KEYCLOAK_DB__DBNAME" \
    -t -A -c "$1"
  local exit_code=$?
  unset PGPASSWORD
  return $exit_code
}


check_db_connection "HASURA"
check_db_connection "KEYCLOAK"

# Fetch ELECTION_ID using the pattern
SQL_FIND_ELECTION_ID="SELECT id::text FROM sequent_backend.election
    WHERE tenant_id = '${TENANT_ID}'::uuid
    AND election_event_id = '${ELECTION_EVENT_ID}'::uuid
    AND (name ILIKE '%${ELECTION_PATTERN}%' OR alias ILIKE '%${ELECTION_PATTERN}%')
    AND name NOT ILIKE '%Test%' -- Assuming we want non-test elections by default
    ORDER BY created_at DESC
    LIMIT 1;"
SQL_FIND_ELECTION_ALIAS="SELECT alias FROM sequent_backend.election
    WHERE tenant_id = '${TENANT_ID}'::uuid
    AND election_event_id = '${ELECTION_EVENT_ID}'::uuid
    AND (name ILIKE '%${ELECTION_PATTERN}%' OR alias ILIKE '%${ELECTION_PATTERN}%')
    AND name NOT ILIKE '%Test%' -- Assuming we want non-test elections by default
    ORDER BY created_at DESC
    LIMIT 1;"

echo -e "\n--- Running SQL (Hasura DB) ---"
echo "$SQL_FIND_ELECTION_ID"
ELECTION_ID=$(run_hasura_psql "$SQL_FIND_ELECTION_ID" | xargs) # xargs to trim whitespace
ELECTION_ALIAS=$(run_hasura_psql "$SQL_FIND_ELECTION_ALIAS" | xargs)


if [ -z "$ELECTION_ID" ]; then
  echo "Error: No election found matching pattern '${ELECTION_PATTERN}' for tenant '${TENANT_ID}' and event '${ELECTION_EVENT_ID}'."
  exit 1
fi

echo "Found ELECTION_ID: ${ELECTION_ID} for pattern '${ELECTION_PATTERN}'"
echo -e "\n--- Preparing monitoring data for Election ID: '${ELECTION_ID}' and Alias: '${ELECTION_ALIAS}'---"

# Constants
AREA_ID_ATTR_NAME="sequent.read-only.area_id"
VALIDATE_ID_ATTR_NAME="sequent.read-only.id-card-number-validated"
VALIDATE_ID_REGISTERED_VOTER="VERIFIED"
LOGIN_EVENT_TYPE="LOGIN"
LOGIN_ERR_EVENT_TYPE="LOGIN_ERROR"


# Initialize aggregate counters
AGG_TOTAL_ELIGIBLE_VOTERS=0
AGG_TOTAL_ENROLLED_VOTERS=0
AGG_TOTAL_AUTHENTICATED=0
AGG_TOTAL_INVALID_USERS_ERRORS=0
AGG_TOTAL_INVALID_PASSWORD_ERRORS=0
AGG_TOTAL_APPROVED=0
AGG_TOTAL_DISAPPROVED=0
AGG_TOTAL_MANUAL_APPROVED=0
AGG_TOTAL_MANUAL_DISAPPROVED=0
AGG_TOTAL_AUTOMATED_APPROVED=0
AGG_TOTAL_AUTOMATED_DISAPPROVED=0

# 1. Fetch areas for the given election_id
# Corresponds to get_areas_by_election_id in Rust
SQL_GET_AREAS_FOR_ELECTION="SELECT DISTINCT ON (a.id) a.id::text
    FROM sequent_backend.area a
    JOIN sequent_backend.area_contest ac ON a.id = ac.area_id AND a.election_event_id = ac.election_event_id AND a.tenant_id = ac.tenant_id
    JOIN sequent_backend.contest c ON ac.contest_id = c.id AND ac.election_event_id = c.election_event_id AND ac.tenant_id = c.tenant_id
    WHERE c.tenant_id = '${TENANT_ID}'::uuid
    AND c.election_event_id = '${ELECTION_EVENT_ID}'::uuid
    AND c.election_id = '${ELECTION_ID}'::uuid;"

echo -e "\n--- Running SQL (Hasura DB) ---"
echo "$SQL_GET_AREAS_FOR_ELECTION"
AREA_IDS_OUTPUT=$(run_hasura_psql "$SQL_GET_AREAS_FOR_ELECTION")

if [ $? -ne 0 ] || [ -z "$AREA_IDS_OUTPUT" ]; then
  echo "Warning: Could not fetch areas for election ${ELECTION_ID}, or no areas found."
  exit 1
else
  OLD_IFS=$IFS
  IFS=$'\n' # Process each line of psql output (each line is an area_id)
  for CURRENT_AREA_ID in $AREA_IDS_OUTPUT; do
    if [ -z "$CURRENT_AREA_ID" ]; then continue; fi
    CURRENT_AREA_ID=$(echo "$CURRENT_AREA_ID" | xargs) # Trim whitespace
    echo "Processing area: ${CURRENT_AREA_ID}"

    # Metric: total_eligible_voters (per area)
    SQL_AREA_ELIGIBLE_VOTERS="SELECT COUNT(DISTINCT u.id) AS total_users
        FROM user_entity AS u
        INNER JOIN realm AS ra ON ra.id = u.realm_id
        WHERE ra.name = '${REALM_NAME}'
        AND u.enabled IS TRUE
        AND (EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = '${AREA_ID_ATTR_NAME}' AND ua.value = '${CURRENT_AREA_ID}'));"
    echo -e "\n--- Running SQL (Keycloak DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_ELIGIBLE_VOTERS"
    AREA_ELIGIBLE_VOTERS=$(run_keycloak_psql "$SQL_AREA_ELIGIBLE_VOTERS")
    [ -z "$AREA_ELIGIBLE_VOTERS" ] && AREA_ELIGIBLE_VOTERS=0
    echo "  Area ${CURRENT_AREA_ID} - Eligible Voters: ${AREA_ELIGIBLE_VOTERS}"
    AGG_TOTAL_ELIGIBLE_VOTERS=$((AGG_TOTAL_ELIGIBLE_VOTERS + AREA_ELIGIBLE_VOTERS))

    # Metric: total_enrolled_voters (per area)
    SQL_AREA_ENROLLED_VOTERS="SELECT COUNT(DISTINCT u.id) AS total_users
        FROM user_entity AS u
        INNER JOIN realm AS ra ON ra.id = u.realm_id
        WHERE ra.name = '${REALM_NAME}'
        AND u.enabled IS TRUE
        AND (EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = '${AREA_ID_ATTR_NAME}' AND ua.value = '${CURRENT_AREA_ID}'))
        AND (EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = '${VALIDATE_ID_ATTR_NAME}' AND ua.value = '${VALIDATE_ID_REGISTERED_VOTER}'));"
    echo -e "\n--- Running SQL (Keycloak DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_ENROLLED_VOTERS"
    AREA_ENROLLED_VOTERS=$(run_keycloak_psql "$SQL_AREA_ENROLLED_VOTERS")
    [ -z "$AREA_ENROLLED_VOTERS" ] && AREA_ENROLLED_VOTERS=0
    echo "  Area ${CURRENT_AREA_ID} - Enrolled Voters: ${AREA_ENROLLED_VOTERS}"
    AGG_TOTAL_ENROLLED_VOTERS=$((AGG_TOTAL_ENROLLED_VOTERS + AREA_ENROLLED_VOTERS))

    # Metric: authentication_stats (per area)
    SQL_AREA_TOTAL_AUTHENTICATED="SELECT COUNT(DISTINCT e.user_id) FROM EVENT_ENTITY as e INNER JOIN realm AS ra ON ra.id = e.realm_id INNER JOIN user_attribute AS us ON us.user_id = e.user_id WHERE ra.name = '${REALM_NAME}' AND e.type = '${LOGIN_EVENT_TYPE}' AND us.name = '${AREA_ID_ATTR_NAME}' AND us.value = '${CURRENT_AREA_ID}';"
    echo -e "\n--- Running SQL (Keycloak DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_TOTAL_AUTHENTICATED"
    AREA_TOTAL_AUTHENTICATED=$(run_keycloak_psql "$SQL_AREA_TOTAL_AUTHENTICATED")
    [ -z "$AREA_TOTAL_AUTHENTICATED" ] && AREA_TOTAL_AUTHENTICATED=0
    echo "  Area ${CURRENT_AREA_ID} - Authenticated: ${AREA_TOTAL_AUTHENTICATED}"
    AGG_TOTAL_AUTHENTICATED=$((AGG_TOTAL_AUTHENTICATED + AREA_TOTAL_AUTHENTICATED))
    
    SQL_AREA_INVALID_USERS="SELECT COUNT(*) FROM EVENT_ENTITY as e INNER JOIN realm AS ra ON ra.id = e.realm_id INNER JOIN user_attribute AS us ON us.user_id = e.user_id WHERE ra.name = '${REALM_NAME}' AND e.type = '${LOGIN_ERR_EVENT_TYPE}' AND e.error = 'user_not_found' AND us.name = '${AREA_ID_ATTR_NAME}' AND us.value = '${CURRENT_AREA_ID}';"
    echo -e "\n--- Running SQL (Keycloak DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_INVALID_USERS"
    AREA_INVALID_USERS_ERRORS=$(run_keycloak_psql "$SQL_AREA_INVALID_USERS")
    [ -z "$AREA_INVALID_USERS_ERRORS" ] && AREA_INVALID_USERS_ERRORS=0
    echo "  Area ${CURRENT_AREA_ID} - Invalid User Errors: ${AREA_INVALID_USERS_ERRORS}"
    AGG_TOTAL_INVALID_USERS_ERRORS=$((AGG_TOTAL_INVALID_USERS_ERRORS + AREA_INVALID_USERS_ERRORS))

    SQL_AREA_INVALID_PASS="SELECT COUNT(*) FROM EVENT_ENTITY as e INNER JOIN realm AS ra ON ra.id = e.realm_id INNER JOIN user_attribute AS us ON us.user_id = e.user_id WHERE ra.name = '${REALM_NAME}' AND e.type = '${LOGIN_ERR_EVENT_TYPE}' AND e.error = 'invalid_user_credentials' AND us.name = '${AREA_ID_ATTR_NAME}' AND us.value = '${CURRENT_AREA_ID}';"
    echo -e "\n--- Running SQL (Keycloak DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_INVALID_PASS"
    AREA_INVALID_PASSWORD_ERRORS=$(run_keycloak_psql "$SQL_AREA_INVALID_PASS")
    [ -z "$AREA_INVALID_PASSWORD_ERRORS" ] && AREA_INVALID_PASSWORD_ERRORS=0
    echo "  Area ${CURRENT_AREA_ID} - Invalid Password Errors: ${AREA_INVALID_PASSWORD_ERRORS}"
    AGG_TOTAL_INVALID_PASSWORD_ERRORS=$((AGG_TOTAL_INVALID_PASSWORD_ERRORS + AREA_INVALID_PASSWORD_ERRORS))

    # Metric: approval_stats (per area)
    SQL_APPROVAL_AREA_BASE="FROM sequent_backend.applications WHERE tenant_id = '${TENANT_ID}'::uuid AND election_event_id = '${ELECTION_EVENT_ID}'::uuid AND area_id = '${CURRENT_AREA_ID}'::uuid"

    SQL_AREA_TOTAL_APPROVED="SELECT COUNT(*) ${SQL_APPROVAL_AREA_BASE} AND status = 'ACCEPTED';"
    echo -e "\n--- Running SQL (Hasura DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_TOTAL_APPROVED"
    AREA_TOTAL_APPROVED=$(run_hasura_psql "$SQL_AREA_TOTAL_APPROVED")
    [ -z "$AREA_TOTAL_APPROVED" ] && AREA_TOTAL_APPROVED=0
    echo "  Area ${CURRENT_AREA_ID} - Approved Applications: ${AREA_TOTAL_APPROVED}"
    AGG_TOTAL_APPROVED=$((AGG_TOTAL_APPROVED + AREA_TOTAL_APPROVED))

    SQL_AREA_MANUAL_APPROVED="SELECT COUNT(*) ${SQL_APPROVAL_AREA_BASE} AND status = 'ACCEPTED' AND verification_type = 'MANUAL';"
    echo -e "\n--- Running SQL (Hasura DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_MANUAL_APPROVED"
    AREA_MANUAL_APPROVED=$(run_hasura_psql "$SQL_AREA_MANUAL_APPROVED")
    [ -z "$AREA_MANUAL_APPROVED" ] && AREA_MANUAL_APPROVED=0
    echo "  Area ${CURRENT_AREA_ID} - Manual Approved: ${AREA_MANUAL_APPROVED}"
    AGG_TOTAL_MANUAL_APPROVED=$((AGG_TOTAL_MANUAL_APPROVED + AREA_MANUAL_APPROVED))

    SQL_AREA_AUTOMATED_APPROVED="SELECT COUNT(*) ${SQL_APPROVAL_AREA_BASE} AND status = 'ACCEPTED' AND verification_type = 'AUTOMATIC';"
    echo -e "\n--- Running SQL (Hasura DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_AUTOMATED_APPROVED"
    AREA_AUTOMATED_APPROVED=$(run_hasura_psql "$SQL_AREA_AUTOMATED_APPROVED")
    [ -z "$AREA_AUTOMATED_APPROVED" ] && AREA_AUTOMATED_APPROVED=0
    echo "  Area ${CURRENT_AREA_ID} - Automated Approved: ${AREA_AUTOMATED_APPROVED}"
    AGG_TOTAL_AUTOMATED_APPROVED=$((AGG_TOTAL_AUTOMATED_APPROVED + AREA_AUTOMATED_APPROVED))

    SQL_AREA_TOTAL_DISAPPROVED="SELECT COUNT(*) ${SQL_APPROVAL_AREA_BASE} AND status = 'REJECTED';"
    echo -e "\n--- Running SQL (Hasura DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_TOTAL_DISAPPROVED"
    AREA_TOTAL_DISAPPROVED=$(run_hasura_psql "$SQL_AREA_TOTAL_DISAPPROVED")
    [ -z "$AREA_TOTAL_DISAPPROVED" ] && AREA_TOTAL_DISAPPROVED=0
    echo "  Area ${CURRENT_AREA_ID} - Disapproved Applications: ${AREA_TOTAL_DISAPPROVED}"
    AGG_TOTAL_DISAPPROVED=$((AGG_TOTAL_DISAPPROVED + AREA_TOTAL_DISAPPROVED))

    SQL_AREA_MANUAL_DISAPPROVED="SELECT COUNT(*) ${SQL_APPROVAL_AREA_BASE} AND status = 'REJECTED' AND verification_type = 'MANUAL';"
    echo -e "\n--- Running SQL (Hasura DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_MANUAL_DISAPPROVED"
    AREA_MANUAL_DISAPPROVED=$(run_hasura_psql "$SQL_AREA_MANUAL_DISAPPROVED")
    [ -z "$AREA_MANUAL_DISAPPROVED" ] && AREA_MANUAL_DISAPPROVED=0
    echo "  Area ${CURRENT_AREA_ID} - Manual Disapproved: ${AREA_MANUAL_DISAPPROVED}"
    AGG_TOTAL_MANUAL_DISAPPROVED=$((AGG_TOTAL_MANUAL_DISAPPROVED + AREA_MANUAL_DISAPPROVED))

    SQL_AREA_AUTOMATED_DISAPPROVED="SELECT COUNT(*) ${SQL_APPROVAL_AREA_BASE} AND status = 'REJECTED' AND verification_type = 'AUTOMATIC';"
    echo -e "\n--- Running SQL (Hasura DB) for Area ${CURRENT_AREA_ID} ---"
    echo "$SQL_AREA_AUTOMATED_DISAPPROVED"
    AREA_AUTOMATED_DISAPPROVED=$(run_hasura_psql "$SQL_AREA_AUTOMATED_DISAPPROVED")
    [ -z "$AREA_AUTOMATED_DISAPPROVED" ] && AREA_AUTOMATED_DISAPPROVED=0
    echo "  Area ${CURRENT_AREA_ID} - Automated Disapproved: ${AREA_AUTOMATED_DISAPPROVED}"
    AGG_TOTAL_AUTOMATED_DISAPPROVED=$((AGG_TOTAL_AUTOMATED_DISAPPROVED + AREA_AUTOMATED_DISAPPROVED))

  done
  IFS=$OLD_IFS
fi


# ---------------------------------------------------------------------------
# Metric: total_voted (for the specific election)
# Corresponds to count_ballots_by_election in Rust
# ---------------------------------------------------------------------------
SQL_ELECTION_TOTAL_VOTED="SELECT COUNT(*) FROM (
    SELECT DISTINCT ON (voter_id_string, area_id) voter_id_string, area_id
    FROM \"sequent_backend\".cast_vote
    WHERE tenant_id = '${TENANT_ID}'::uuid
    AND election_event_id = '${ELECTION_EVENT_ID}'::uuid
    AND election_id = '${ELECTION_ID}'::uuid
    ORDER BY voter_id_string, area_id, created_at DESC
) AS latest_votes;"

echo -e "\n--- Running SQL (Hasura DB) ---"
echo "$SQL_ELECTION_TOTAL_VOTED"
TOTAL_VOTED_FOR_ELECTION=$(run_hasura_psql "$SQL_ELECTION_TOTAL_VOTED")
[ -z "$TOTAL_VOTED_FOR_ELECTION" ] && TOTAL_VOTED_FOR_ELECTION=0

echo -e "\n\n------- FINAL DATA ----------\n\n"

echo "total_eligible_voters: ${AGG_TOTAL_ELIGIBLE_VOTERS}"
echo "total_enrolled_voters: ${AGG_TOTAL_ENROLLED_VOTERS}" # This is sum of enrolled per area for this election
echo "authentication_stats.total_authenticated: ${AGG_TOTAL_AUTHENTICATED}"
# total_not_authenticated for a specific election and its areas would be AGG_TOTAL_ENROLLED_VOTERS - AGG_TOTAL_AUTHENTICATED
echo "authentication_stats.total_not_authenticated: $((AGG_TOTAL_ENROLLED_VOTERS - AGG_TOTAL_AUTHENTICATED))"
echo "authentication_stats.total_invalid_users_errors: ${AGG_TOTAL_INVALID_USERS_ERRORS}"
echo "authentication_stats.total_invalid_password_errors: ${AGG_TOTAL_INVALID_PASSWORD_ERRORS}"
echo "approval_stats.total_approved: ${AGG_TOTAL_APPROVED}"
echo "approval_stats.total_disapproved: ${AGG_TOTAL_DISAPPROVED}"
echo "approval_stats.total_manual_approved: ${AGG_TOTAL_MANUAL_APPROVED}"
echo "approval_stats.total_manual_disapproved: ${AGG_TOTAL_MANUAL_DISAPPROVED}"
echo "approval_stats.total_automated_approved: ${AGG_TOTAL_AUTOMATED_APPROVED}"
echo "approval_stats.total_automated_disapproved: ${AGG_TOTAL_AUTOMATED_DISAPPROVED}"

echo "total_voted: ${TOTAL_VOTED_FOR_ELECTION}"


echo -e "\nMonitoring data preparation complete."

