#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Test script to verify tally_session_resolution table is working

set -e

echo "Testing tally_session_resolution GraphQL API..."

# Test 1: Query empty table
echo "Test 1: Query empty table"
curl -s -X POST http://hasura:8080/v1/graphql \
  -H "X-Hasura-Admin-Secret: admin" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { sequent_backend_tally_session_resolution { id resolution_type status } }"
  }' | jq '.data.sequent_backend_tally_session_resolution | length'

# Test 2: Insert a test resolution (using SQL since we don't have a tally session yet)
echo ""
echo "Test 2: Insert test data"
docker exec postgres psql -U postgres -d postgres -c "
  -- First, get a real tally_session ID
  DO \$\$
  DECLARE
    test_tenant_id uuid;
    test_event_id uuid;
    test_session_id uuid;
  BEGIN
    -- Get an existing tally session
    SELECT tenant_id, election_event_id, id
    INTO test_tenant_id, test_event_id, test_session_id
    FROM sequent_backend.tally_session
    LIMIT 1;

    IF test_session_id IS NOT NULL THEN
      -- Insert a test resolution
      INSERT INTO sequent_backend.tally_session_resolution
        (tenant_id, election_event_id, tally_session_id, resolution_type, status, resolution_data)
      VALUES
        (test_tenant_id, test_event_id, test_session_id, 'irv_tie_break', 'pending',
         '{\"round_number\": 1, \"tied_candidate_ids\": [\"test-1\", \"test-2\"]}'::jsonb)
      ON CONFLICT DO NOTHING;

      RAISE NOTICE 'Test resolution inserted successfully';
    ELSE
      RAISE NOTICE 'No tally sessions found - skipping insert test';
    END IF;
  END \$\$;
" 2>&1 | grep -E "NOTICE|INSERT"

# Test 3: Query the inserted data
echo ""
echo "Test 3: Query inserted data"
curl -s -X POST http://hasura:8080/v1/graphql \
  -H "X-Hasura-Admin-Secret: admin" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { sequent_backend_tally_session_resolution(where: {status: {_eq: \"pending\"}}) { id resolution_type status resolution_data } }"
  }' | jq '.data.sequent_backend_tally_session_resolution'

# Test 4: Test relationship to tally_session
echo ""
echo "Test 4: Test relationship to tally_session"
curl -s -X POST http://hasura:8080/v1/graphql \
  -H "X-Hasura-Admin-Secret: admin" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { sequent_backend_tally_session_resolution(limit: 1) { id resolution_type tally_session { id tally_type } } }"
  }' | jq '.data.sequent_backend_tally_session_resolution[0]'

echo ""
echo "✅ All tests completed!"
