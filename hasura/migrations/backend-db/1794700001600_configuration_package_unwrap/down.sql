-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

DROP INDEX sequent_backend.configuration_package_staging_credentials_expiry;
ALTER TABLE sequent_backend.configuration_package_staging
    DROP CONSTRAINT configuration_package_staging_unwrap_needs_uploader,
    DROP CONSTRAINT configuration_package_staging_credentials_need_unwrap,
    DROP COLUMN credentials_expires_at,
    DROP COLUMN unwrapped_id,
    DROP COLUMN uploaded_by;
