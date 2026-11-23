-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. These functions hold no rows of their own; the
-- grants they read are protected by the rollback of 1794700001100.
DROP FUNCTION sequent_backend.operational_signing_recorded_device_profile(uuid, uuid, uuid, text, uuid);
DROP FUNCTION sequent_backend.operational_signing_settle_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text, bytea);
DROP FUNCTION sequent_backend.operational_signing_claim_device_from_reference(uuid, uuid, uuid, text, uuid, uuid, text);
DROP FUNCTION sequent_backend.operational_initialize_claim_current_device(uuid, uuid, uuid, text, uuid, uuid, text, text, text);
DROP FUNCTION sequent_backend.operational_signing_get_device_grant(uuid, uuid, uuid, text, uuid, uuid);
DROP FUNCTION sequent_backend.operational_initialize_get_ca_enrollment(uuid, uuid, uuid, text, uuid, uuid, text);
DROP FUNCTION sequent_backend.operational_initialize_ca_version(uuid, uuid, text, text);
DROP FUNCTION sequent_backend.operational_signing_device_caller(sequent_backend.operational_signing_flow);
