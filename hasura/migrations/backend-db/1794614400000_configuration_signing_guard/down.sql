-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS "guard_configuration_signing" ON "sequent_backend"."tenant";
DROP FUNCTION IF EXISTS "sequent_backend"."guard_configuration_signing"();
DROP FUNCTION IF EXISTS "sequent_backend"."configuration_signing_change_allowed"(uuid);
