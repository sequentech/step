CREATE TABLE "sequent_backend"."electoral_log_checkpoint"
(
    "id"                uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "board_name"        text        NOT NULL,
    "log_uid"           uuid        NOT NULL,
    "tree_size"         bigint      NOT NULL CHECK ("tree_size" >= 0),
    "root"              text        NOT NULL CHECK ("root" ~ '^[0-9a-f]{64}$'),
    "reason"            text        NOT NULL CHECK ("reason" IN ('VOTING_CLOSED', 'TALLY_COMPLETED')),
    "signer_pk"         text        NOT NULL,
    "signature"         text        NOT NULL,
    "created_at"        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT "electoral_log_checkpoint_log_uid_tree_size_key"
        UNIQUE ("tenant_id", "election_event_id", "log_uid", "tree_size")
);
COMMENT ON TABLE "sequent_backend"."electoral_log_checkpoint" IS
    'Signed checkpoints of election event electoral logs, kept outside the electoral-log database. The application only inserts rows.';
COMMENT ON COLUMN "sequent_backend"."electoral_log_checkpoint"."log_uid" IS
    'Identity of the Trellis log: the event''s board, or a sealed log it continues after an import.';
