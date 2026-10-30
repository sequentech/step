-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Signed electoral log events of accepted votes, stored in the vote's
-- transaction and removed once they are published to the electoral log
-- queue. Internal to Windmill; not tracked by Hasura.
CREATE TABLE sequent_backend.electoral_log_outbox (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    input jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX electoral_log_outbox_created_at_idx
    ON sequent_backend.electoral_log_outbox (created_at);
