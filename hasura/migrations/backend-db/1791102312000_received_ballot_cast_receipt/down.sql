-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.cast_vote DROP COLUMN cast_receipt_signature;

ALTER TABLE sequent_backend.received_ballot
    DROP CONSTRAINT received_ballot_cast_receipt_check,
    DROP COLUMN cast_receipt_signature,
    DROP COLUMN cast_signature;
