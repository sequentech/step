-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Server-side unwrap of an encrypted configuration package that carries
-- messaging credentials (harvest configuration_package_intake).
--
-- uploaded_by: the verified token's user ID (hasura_claims.user_id, the ID
--   the audit actor and Apply's task carry) that requested this protected
--   staging row. Only that user may unwrap it, and only that user's Preview
--   and Apply may use the credentials staged with it.
-- unwrapped_id: names a server-only sibling object holding the unwrapped
--   signed ZIP. It is never presigned, so the browser's upload URL can't
--   overwrite it. Set at most once, in the transaction that stages the
--   credentials in the vault; readers use it in place of delivery.zip.
-- credentials_expires_at: when the staged credentials stop being accepted
--   and may be swept from the vault. Cleared once Apply consumes them.
--
-- All nullable, no backfill: existing rows and ordinary deliveries (unwrapped
-- in the browser, no credentials) behave exactly as before. A row without
-- uploaded_by can't be unwrapped server side. Still not tracked by Hasura.
ALTER TABLE sequent_backend.configuration_package_staging
    ADD COLUMN uploaded_by text,
    ADD COLUMN unwrapped_id uuid UNIQUE,
    ADD COLUMN credentials_expires_at timestamptz,
    ADD CONSTRAINT configuration_package_staging_credentials_need_unwrap
        CHECK (credentials_expires_at IS NULL OR unwrapped_id IS NOT NULL),
    ADD CONSTRAINT configuration_package_staging_unwrap_needs_uploader
        CHECK (unwrapped_id IS NULL OR uploaded_by IS NOT NULL);

-- Sweeping expired staged credentials, per tenant.
CREATE INDEX configuration_package_staging_credentials_expiry
    ON sequent_backend.configuration_package_staging (tenant_id, credentials_expires_at)
    WHERE credentials_expires_at IS NOT NULL;
