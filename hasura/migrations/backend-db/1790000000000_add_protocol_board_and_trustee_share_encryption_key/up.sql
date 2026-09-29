-- Trustees of the new core hold two keys: the Ed25519 key they sign board
-- messages with, already stored in "public_key", and an ElGamal key their peers
-- encrypt their key generation shares to, which is this new column.
ALTER TABLE "sequent_backend"."trustee"
    ADD COLUMN "share_encryption_public_key" TEXT;

COMMENT ON COLUMN "sequent_backend"."trustee"."share_encryption_public_key" IS
    'Base64 of the canonical group element bytes. NULL until the trustee has a key.';

-- One row per board the platform has created on the board service (b4).
-- The board service is untrusted, so this is what says which Configuration each
-- board must serve.
CREATE TABLE "sequent_backend"."protocol_board"
(
    "id"                uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "parent_id"         uuid        NULL,
    "keys_ceremony_id"  uuid        NOT NULL,
    "tally_session_id"  uuid        NULL,
    "batch"             integer     NULL,
    "name"              text        NOT NULL,
    "manager_message"   bytea       NOT NULL,
    "created_at"        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    UNIQUE ("name"),
    UNIQUE ("tally_session_id", "batch"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("election_event_id")
        REFERENCES "sequent_backend"."election_event" ("id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("parent_id")
        REFERENCES "sequent_backend"."protocol_board" ("id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("keys_ceremony_id", "tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."keys_ceremony" ("id", "tenant_id", "election_event_id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("tally_session_id", "tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."tally_session" ("id", "tenant_id", "election_event_id")
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK (
        ("parent_id" IS NULL) = ("tally_session_id" IS NULL)
        AND ("parent_id" IS NULL) = ("batch" IS NULL)
    )
);

COMMENT ON COLUMN "sequent_backend"."protocol_board"."parent_id" IS
    'The parent board of a child board, or NULL for parent. A child board is any tally board, and its parent is dkg.';

COMMENT ON COLUMN "sequent_backend"."protocol_board"."tally_session_id" IS
    'The tally session of a tally board, or NULL for dkg. Set together with parent_id and batch.';

COMMENT ON COLUMN "sequent_backend"."protocol_board"."batch" IS
    'The ballot batch of a tally board, or NULL for dkg: the session_id of its tally session contest plus the weight batch offset.';

COMMENT ON COLUMN "sequent_backend"."protocol_board"."manager_message" IS
    'The canonical bytes of the message published by the protocol manager it when the ceremony was created. Configuration for dkg, and Ballots for tally.';

CREATE UNIQUE INDEX "protocol_board_one_root_per_ceremony_idx"
    ON "sequent_backend"."protocol_board" ("keys_ceremony_id")
    WHERE parent_id IS NULL;
CREATE INDEX "protocol_board_keys_ceremony_id_idx"
    ON "sequent_backend"."protocol_board" ("keys_ceremony_id");
