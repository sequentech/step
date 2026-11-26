-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DO $$ BEGIN
    IF sequent_backend.operational_signing_demo_fixture_id('10000000-0000-4000-8000-000000000001','10000000-0000-4000-8000-000000000002','initialize-voting','softhsm-test') IS NOT NULL THEN
        RAISE EXCEPTION 'restored demo marker admitted software on another DB';
    END IF;
END $$;
