-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
REVOKE EXECUTE ON FUNCTION sequent_backend.operational_setup_certificate_material(uuid,uuid,uuid,uuid)
    FROM sequent_os_product_setup_ca;
DROP FUNCTION sequent_backend.operational_attach_requested_setup_certificate(uuid,uuid,uuid,uuid,uuid,text,text[]);
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_product_setup_ca;
