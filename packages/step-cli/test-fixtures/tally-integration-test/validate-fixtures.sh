#!/bin/bash
# Validate test fixtures structure and completeness
# This script checks that all fixtures have the required properties that were missing from CLI exports

set -e

FIXTURES_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$FIXTURES_DIR"

FIXTURES=(
  "election-fixture.json"
  "election-fixture-minimal.json"
  "election-fixture-irv.json"
)

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

check_property() {
  local fixture=$1
  local jq_query=$2
  local property_name=$3
  local expected=$4

  TOTAL_TESTS=$((TOTAL_TESTS + 1))

  result=$(jq -r "$jq_query" "$fixture" 2>/dev/null || echo "ERROR")

  if [ "$result" = "$expected" ]; then
    echo -e "  ${GREEN}✅${NC} $property_name"
    PASSED_TESTS=$((PASSED_TESTS + 1))
  else
    echo -e "  ${RED}❌${NC} $property_name (got: $result, expected: $expected)"
    FAILED_TESTS=$((FAILED_TESTS + 1))
  fi
}

check_not_null() {
  local fixture=$1
  local jq_query=$2
  local property_name=$3

  TOTAL_TESTS=$((TOTAL_TESTS + 1))

  result=$(jq -r "$jq_query" "$fixture" 2>/dev/null || echo "ERROR")

  if [ "$result" != "null" ] && [ "$result" != "ERROR" ]; then
    echo -e "  ${GREEN}✅${NC} $property_name is present"
    PASSED_TESTS=$((PASSED_TESTS + 1))
  else
    echo -e "  ${RED}❌${NC} $property_name is missing (null)"
    FAILED_TESTS=$((FAILED_TESTS + 1))
  fi
}

check_has_i18n() {
  local fixture=$1
  local jq_query=$2
  local property_name=$3

  TOTAL_TESTS=$((TOTAL_TESTS + 1))

  result=$(jq -r "$jq_query" "$fixture" 2>/dev/null || echo "ERROR")

  if [ "$result" != "null" ] && [ "$result" != "ERROR" ] && [ "$result" -gt 0 ]; then
    echo -e "  ${GREEN}✅${NC} $property_name has i18n (languages: $result)"
    PASSED_TESTS=$((PASSED_TESTS + 1))
  else
    echo -e "  ${RED}❌${NC} $property_name missing i18n"
    FAILED_TESTS=$((FAILED_TESTS + 1))
  fi
}

echo -e "${BLUE}=====================================${NC}"
echo -e "${BLUE}  Test Fixture Validation${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""

for fixture in "${FIXTURES[@]}"; do
  if [ ! -f "$fixture" ]; then
    echo -e "${RED}❌ Fixture not found: $fixture${NC}"
    continue
  fi

  echo -e "${YELLOW}📄 Validating: $fixture${NC}"
  echo ""

  # Basic JSON validity
  if jq -e '.' "$fixture" > /dev/null 2>&1; then
    echo -e "  ${GREEN}✅${NC} Valid JSON syntax"
    PASSED_TESTS=$((PASSED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
  else
    echo -e "  ${RED}❌${NC} Invalid JSON syntax"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    continue
  fi

  # Check basic structure
  check_property "$fixture" '.election_event.id | type' "election_event.id exists" "string"
  check_property "$fixture" '.election_event.name | type' "election_event.name exists" "string"
  check_property "$fixture" '.election_event.tenant_id' "election_event.tenant_id" "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"

  # Critical properties that were missing from CLI exports
  echo ""
  echo -e "  ${BLUE}Checking properties missing from CLI exports:${NC}"

  # 1. election_event.presentation
  check_not_null "$fixture" '.election_event.presentation' "election_event.presentation"
  check_has_i18n "$fixture" '.election_event.presentation.i18n | keys | length' "election_event.presentation.i18n"
  check_not_null "$fixture" '.election_event.presentation.language_conf' "election_event.presentation.language_conf"

  # 2. elections[].presentation
  check_not_null "$fixture" '.elections[0].presentation' "elections[0].presentation"
  check_has_i18n "$fixture" '.elections[0].presentation.i18n | keys | length' "elections[0].presentation.i18n"

  # 3. elections[].status
  check_not_null "$fixture" '.elections[0].status' "elections[0].status"
  check_not_null "$fixture" '.elections[0].status.voting_status' "elections[0].status.voting_status"

  # 4. elections[].voting_channels
  check_not_null "$fixture" '.elections[0].voting_channels' "elections[0].voting_channels"
  check_property "$fixture" '.elections[0].voting_channels.online' "elections[0].voting_channels.online" "true"

  # 5. contests[].presentation
  check_not_null "$fixture" '.contests[0].presentation' "contests[0].presentation"
  check_has_i18n "$fixture" '.contests[0].presentation.i18n | keys | length' "contests[0].presentation.i18n"
  check_not_null "$fixture" '.contests[0].presentation.candidates_order' "contests[0].presentation.candidates_order"

  # 6. candidates[].presentation
  check_not_null "$fixture" '.candidates[0].presentation' "candidates[0].presentation"
  check_has_i18n "$fixture" '.candidates[0].presentation.i18n | keys | length' "candidates[0].presentation.i18n"

  # 7. candidates[].is_public
  check_property "$fixture" '.candidates[0].is_public' "candidates[0].is_public (not null)" "false"

  # 8. areas[].presentation
  check_not_null "$fixture" '.areas[0].presentation' "areas[0].presentation"
  check_not_null "$fixture" '.areas[0].presentation.allow_early_voting' "areas[0].presentation.allow_early_voting"

  echo ""
  echo -e "  ${BLUE}Checking entity counts:${NC}"
  elections_count=$(jq -r '.elections | length' "$fixture")
  contests_count=$(jq -r '.contests | length' "$fixture")
  candidates_count=$(jq -r '.candidates | length' "$fixture")
  areas_count=$(jq -r '.areas | length' "$fixture")

  echo -e "  ${GREEN}ℹ${NC}  Elections: $elections_count"
  echo -e "  ${GREEN}ℹ${NC}  Contests: $contests_count"
  echo -e "  ${GREEN}ℹ${NC}  Candidates: $candidates_count"
  echo -e "  ${GREEN}ℹ${NC}  Areas: $areas_count"

  echo ""
done

echo -e "${BLUE}=====================================${NC}"
echo -e "${BLUE}  Validation Summary${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""
echo -e "Total tests: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
  echo -e "${GREEN}✅ All fixtures are valid and complete!${NC}"
  exit 0
else
  echo -e "${RED}❌ Some fixtures have issues. Please review the errors above.${NC}"
  exit 1
fi
