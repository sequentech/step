-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TRIGGER guard_configuration_signing_settings ON sequent_backend.tenant;
DROP FUNCTION sequent_backend.guard_configuration_signing_settings();
