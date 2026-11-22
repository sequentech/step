-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Every cast, legacy events included, asks whether its event requires
-- sealing. The answer (the flag and the event's public seal key, which is
-- also pinned in the electoral log) is public, so any login that can use the
-- schema may read it: a runtime login without sequent_os_approve must keep
-- casting in legacy events. Requiring and publishing stay with
-- sequent_os_cast_vote_key_publisher; the tables stay insert-only and
-- unreadable.
GRANT EXECUTE ON FUNCTION sequent_backend.cast_vote_seal_state(uuid, uuid) TO PUBLIC;
