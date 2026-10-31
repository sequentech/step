-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER guard_publication_style_material ON sequent_backend.ballot_style;
DROP FUNCTION sequent_backend.guard_publication_style_material();
DROP TRIGGER guard_publication_authority ON sequent_backend.ballot_publication;
DROP FUNCTION sequent_backend.guard_publication_authority();
