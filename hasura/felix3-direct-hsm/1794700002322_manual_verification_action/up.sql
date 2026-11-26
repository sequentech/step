-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));

-- CREATE OR REPLACE preserves the existing owner and EXECUTE privileges.
CREATE OR REPLACE FUNCTION felix3_initialize_direct.known_action(p_action text)
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT SET search_path=pg_catalog,pg_temp AS $$
    SELECT p_action COLLATE "C" IN (
        'initialize-voting', 'open-voting', 'close-voting',
        'generate-election-returns', 'generate-reports', 'transmit-results',
        'approve-voter', 'authorize-manual-verification', 'approve-configuration',
        'key-ceremony', 'tally-key'
    );
$$;
