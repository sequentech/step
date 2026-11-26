-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Never remove the authority records underneath outstanding signed credentials.
DO $$ BEGIN
    IF EXISTS (SELECT FROM sequent_backend.manual_verification_issuance) THEN
        RAISE EXCEPTION 'Manual-verification issuance history must be retained';
    END IF;
END $$;
DROP TABLE sequent_backend.manual_verification_issuance;
