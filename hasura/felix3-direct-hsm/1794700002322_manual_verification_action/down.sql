-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

-- The narrower function may not invalidate a latched request or recorded proof.
DO $$
BEGIN
    IF EXISTS (SELECT FROM felix3_initialize_direct.action_request_birth WHERE action = 'authorize-manual-verification')
       OR EXISTS (SELECT FROM felix3_initialize_direct.action_request_scope WHERE action = 'authorize-manual-verification')
       OR EXISTS (SELECT FROM felix3_initialize_direct.action_reservation WHERE action = 'authorize-manual-verification')
       OR EXISTS (SELECT FROM sequent_backend.signing_approval WHERE action_direct_proof->>'action' = 'authorize-manual-verification')
    THEN RAISE EXCEPTION 'manual verification signing has been used; retain its evidence and roll forward';
    END IF;
END;
$$;

-- CREATE OR REPLACE preserves the existing owner and EXECUTE privileges.
CREATE OR REPLACE FUNCTION felix3_initialize_direct.known_action(p_action text)
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
    SELECT p_action COLLATE "C" IN (
        'initialize-voting', 'open-voting', 'close-voting',
        'generate-election-returns', 'generate-reports', 'transmit-results',
        'approve-voter', 'approve-configuration', 'key-ceremony',
        'tally-key'
    );
$$;
