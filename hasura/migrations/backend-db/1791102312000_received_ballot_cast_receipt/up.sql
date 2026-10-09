-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The voter's signature that cast a received ballot, and the receipt the
-- ballot box signed for that cast.
ALTER TABLE sequent_backend.received_ballot
    ADD COLUMN cast_signature text,
    ADD COLUMN cast_receipt_signature text,
    ADD CONSTRAINT received_ballot_cast_receipt_check CHECK (
        (status = 'cast') = (cast_at IS NOT NULL)
        AND (cast_signature IS NULL) = (cast_receipt_signature IS NULL)
        AND (cast_signature IS NULL OR status = 'cast')
    );

ALTER TABLE sequent_backend.cast_vote ADD COLUMN cast_receipt_signature text;
