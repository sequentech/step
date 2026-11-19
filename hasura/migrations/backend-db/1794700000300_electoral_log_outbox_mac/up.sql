-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A system-signed pending statement's MAC (HMAC-SHA256 under the tenant's
-- electoral-log-pending-mac vault secret): the relay and the batch task refuse a
-- row without a valid one, so whoever can write rows but can't read the vault
-- (a database-only compromise) can't have a board sign anything.
ALTER TABLE sequent_backend.electoral_log_outbox ADD COLUMN mac bytea;
