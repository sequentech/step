-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE OR REPLACE FUNCTION sequent_backend.audit_storage_allows_voters()
RETURNS boolean LANGUAGE sql STABLE AS $$ SELECT false; $$;
DROP TABLE sequent_backend.audit_storage_notice;
DROP TABLE sequent_backend.audit_storage_status;
DROP TABLE sequent_backend.security_notification;
