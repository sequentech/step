-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS "check_report_tenant_binding"
ON "sequent_backend"."report";

DROP FUNCTION IF EXISTS "sequent_backend"."check_report_tenant_binding"();
