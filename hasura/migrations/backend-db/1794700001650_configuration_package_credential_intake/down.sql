-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Preserve in-flight or already bound staging, including rows adopted from
-- the provisional migration. Roll back only after draining the application.
LOCK TABLE sequent_backend.configuration_package_staging IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.configuration_package_staging
        WHERE uploaded_by IS NOT NULL OR unwrapped_id IS NOT NULL OR credentials_expires_at IS NOT NULL)
    THEN
        RAISE EXCEPTION 'Configuration package staging retains bound uploads; refuse credential intake rollback';
    END IF;
END $$;

DROP INDEX sequent_backend.configuration_package_staging_credentials_expiry;
ALTER TABLE sequent_backend.configuration_package_staging
    DROP CONSTRAINT configuration_package_staging_unwrap_needs_uploader,
    DROP CONSTRAINT configuration_package_staging_credentials_need_unwrap,
    DROP COLUMN credentials_expires_at,
    DROP COLUMN unwrapped_id,
    DROP COLUMN uploaded_by;
