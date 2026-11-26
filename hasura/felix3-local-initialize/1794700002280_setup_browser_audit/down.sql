-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TRIGGER local_setup_browser_audit ON felix3_initialize_setup.attempt;
DROP FUNCTION sequent_backend.audit_local_setup_browser_transition();
