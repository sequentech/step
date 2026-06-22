CREATE TABLE "sequent_backend"."tally_results_publication" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id" uuid NOT NULL,
    "election_event_id" uuid NOT NULL,
    "tally_session_id" uuid NOT NULL,
    "tally_session_execution_id" uuid NOT NULL,
    "results_event_id" uuid NOT NULL,
    "task_execution_id" uuid,
    "route_scope" text NOT NULL,
    "route_election_id" uuid,
    "election_ids" uuid[] NOT NULL DEFAULT '{}',
    "access" text NOT NULL,
    "visibility_scope" text NOT NULL,
    "published_contest_ids" jsonb NOT NULL DEFAULT '[]'::jsonb,
    "contest_publication_state" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "documents" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "manifest" jsonb,
    "publication_status" text NOT NULL,
    "version" integer NOT NULL,
    "error_message" text,
    "published_at" timestamptz,
    "published_by_user_id" uuid,
    "revoked_at" timestamptz,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id", "tenant_id", "election_event_id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE restrict
        ON DELETE restrict,
    FOREIGN KEY ("election_event_id")
        REFERENCES "sequent_backend"."election_event" ("id")
        ON UPDATE restrict
        ON DELETE restrict,
    FOREIGN KEY ("tally_session_id", "tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."tally_session" ("id", "tenant_id", "election_event_id")
        ON UPDATE restrict
        ON DELETE restrict,
    FOREIGN KEY ("tally_session_execution_id", "tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."tally_session_execution" ("id", "tenant_id", "election_event_id")
        ON UPDATE restrict
        ON DELETE restrict,
    FOREIGN KEY ("results_event_id", "tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."results_event" ("id", "tenant_id", "election_event_id")
        ON UPDATE restrict
        ON DELETE restrict,
    CHECK ("route_scope" IN ('event', 'election')),
    CHECK (
        ("route_scope" = 'event' AND "route_election_id" IS NULL)
        OR ("route_scope" = 'election' AND "route_election_id" IS NOT NULL)
    ),
    CHECK ("access" IN ('public', 'authenticated')),
    CHECK ("visibility_scope" IN ('full_event', 'area_based')),
    CHECK (
        ("access" = 'authenticated')
        OR ("visibility_scope" = 'full_event')
    ),
    CHECK (
        "publication_status" IN (
            'Draft',
            'Publishing',
            'Published',
            'Superseded',
            'Revoked',
            'Failed'
        )
    )
);

CREATE INDEX "tally_results_publication_event_idx"
    ON "sequent_backend"."tally_results_publication" (
        "tenant_id",
        "election_event_id",
        "route_scope",
        "route_election_id",
        "publication_status"
    );

CREATE UNIQUE INDEX "tally_results_publication_active_event_idx"
    ON "sequent_backend"."tally_results_publication" (
        "tenant_id",
        "election_event_id",
        "route_scope"
    )
    WHERE "route_scope" = 'event'
      AND "publication_status" = 'Published'
      AND "revoked_at" IS NULL;

CREATE UNIQUE INDEX "tally_results_publication_active_election_idx"
    ON "sequent_backend"."tally_results_publication" (
        "tenant_id",
        "election_event_id",
        "route_scope",
        "route_election_id"
    )
    WHERE "route_scope" = 'election'
      AND "publication_status" = 'Published'
      AND "revoked_at" IS NULL;

CREATE UNIQUE INDEX "tally_results_publication_route_version_idx"
    ON "sequent_backend"."tally_results_publication" (
        "tenant_id",
        "election_event_id",
        "route_scope",
        (COALESCE("route_election_id", '00000000-0000-0000-0000-000000000000'::uuid)),
        "version"
    );
