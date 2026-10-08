CREATE TABLE "sequent_backend"."approval_matrix"
(
    "id"                  uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"           uuid        NOT NULL,
    "election_event_id"   uuid        NOT NULL,
    "version"             integer     NOT NULL CHECK ("version" > 0),
    "compared_fields"     jsonb       NOT NULL,
    "rules"               jsonb       NOT NULL,
    "otherwise"           jsonb       NOT NULL,
    "sha256"              text        NOT NULL,
    "created_at"          timestamptz NOT NULL DEFAULT now(),
    "created_by"          text,
    "created_by_username" text,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE RESTRICT
        ON DELETE CASCADE,
    UNIQUE ("tenant_id", "election_event_id", "version")
);

COMMENT ON TABLE "sequent_backend"."approval_matrix" IS
    'Versions of an election event''s enrollment approval matrix. Rows are never changed: saving the matrix adds a version, and the latest version decides new enrollments.';

CREATE FUNCTION "sequent_backend"."approval_matrix_is_immutable"() RETURNS trigger AS
$$
BEGIN
    RAISE EXCEPTION 'approval matrix versions are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER "approval_matrix_immutable"
    BEFORE UPDATE
    ON "sequent_backend"."approval_matrix"
    FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."approval_matrix_is_immutable"();
