-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- One fingerprint per contest ciphertext of the ballot. Rows cast before this
-- column existed keep it NULL.
ALTER TABLE sequent_backend.cast_vote
  ADD COLUMN IF NOT EXISTS ciphertext_fingerprints text[];

CREATE INDEX IF NOT EXISTS cast_vote_ciphertext_fingerprints_idx
ON sequent_backend.cast_vote USING gin (ciphertext_fingerprints);
