-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Cast votes are stored in each election event's ballot box, in its
-- electoral-log database, which enforces the revote and area rules itself.
DROP TABLE "sequent_backend"."cast_vote";
DROP FUNCTION IF EXISTS public.check_revote_limit();
