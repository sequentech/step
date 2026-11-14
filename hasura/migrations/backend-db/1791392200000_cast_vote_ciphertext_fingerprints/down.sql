-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP INDEX IF EXISTS sequent_backend.cast_vote_ciphertext_fingerprints_idx;

ALTER TABLE sequent_backend.cast_vote
  DROP COLUMN IF EXISTS ciphertext_fingerprints;
