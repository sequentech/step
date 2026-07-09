CREATE TABLE "sequent_backend"."tally_sheet_import" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id" uuid NOT NULL,
    "election_event_id" uuid NOT NULL,
    "source_document_id" uuid NOT NULL,
    "source_file_name" text NULL,
    "source_sha256" text NULL,
    "source_format" text NOT NULL,
    "selected_channel" text NOT NULL,
    "status" text NOT NULL DEFAULT 'PENDING_REVIEW',
    "created_by_user_id" text NOT NULL,
    "annotations" jsonb,
    "labels" jsonb,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "last_updated_at" timestamptz NOT NULL DEFAULT now(),
    "summary" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "validation_report" jsonb NULL,
    "canonical_csv_sha256" text NULL,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id", "election_event_id") REFERENCES "sequent_backend"."election_event" ("tenant_id", "id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("source_document_id") REFERENCES "sequent_backend"."document" ("id") ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX "tally_sheet_import_event_idx" ON "sequent_backend"."tally_sheet_import" ("tenant_id", "election_event_id", "created_at" DESC);
CREATE INDEX "tally_sheet_import_document_idx" ON "sequent_backend"."tally_sheet_import" ("source_document_id");

ALTER TABLE "sequent_backend"."tally_sheet"
    ADD COLUMN "import_id" uuid NULL;

ALTER TABLE "sequent_backend"."tally_sheet"
    ADD CONSTRAINT "tally_sheet_import_id_fkey"
    FOREIGN KEY ("import_id") REFERENCES "sequent_backend"."tally_sheet_import" ("id") ON UPDATE RESTRICT ON DELETE RESTRICT;

CREATE TABLE "sequent_backend"."tally_sheet_import_item" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id" uuid NOT NULL,
    "election_event_id" uuid NOT NULL,
    "import_id" uuid NOT NULL,
    "election_id" uuid NOT NULL,
    "area_id" uuid NOT NULL,
    "contest_id" uuid NOT NULL,
    "channel" text NOT NULL,
    "generated_tally_sheet_id" uuid NULL,
    "baseline_approved_tally_sheet_id" uuid NULL,
    "baseline_approved_version" integer NULL,
    "baseline_content_hash" text NULL,
    "incoming_content_hash" text NOT NULL,
    "change_type" text NOT NULL,
    "status" text NOT NULL DEFAULT 'PENDING_REVIEW',
    "previous_csv" text NULL,
    "incoming_csv" text NOT NULL,
    "source_refs" jsonb NULL,
    "validation_warnings" jsonb NULL,
    "annotations" jsonb,
    "labels" jsonb,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "last_updated_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    FOREIGN KEY ("import_id") REFERENCES "sequent_backend"."tally_sheet_import" ("id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("generated_tally_sheet_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."tally_sheet" ("id", "tenant_id", "election_event_id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("baseline_approved_tally_sheet_id", "tenant_id", "election_event_id") REFERENCES "sequent_backend"."tally_sheet" ("id", "tenant_id", "election_event_id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("tenant_id", "election_event_id", "election_id") REFERENCES "sequent_backend"."election" ("tenant_id", "election_event_id", "id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("tenant_id", "election_event_id", "area_id") REFERENCES "sequent_backend"."area" ("tenant_id", "election_event_id", "id") ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY ("tenant_id", "election_event_id", "contest_id") REFERENCES "sequent_backend"."contest" ("tenant_id", "election_event_id", "id") ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE UNIQUE INDEX "tally_sheet_import_item_ballot_box_idx" ON "sequent_backend"."tally_sheet_import_item" ("import_id", "election_id", "area_id", "contest_id", "channel");
CREATE INDEX "tally_sheet_import_item_event_idx" ON "sequent_backend"."tally_sheet_import_item" ("tenant_id", "election_event_id", "import_id");

-- Follow-up fix for migration 1763981518000 (already shipped, left untouched
-- since environments may have applied it already): the original migration
-- blindly set every row's "version" to 1 instead of numbering versions per
-- ballot box, and its unique index didn't exclude soft-deleted rows.
DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_uniq_version";

WITH ranked AS (
  SELECT id, ROW_NUMBER() OVER (
    PARTITION BY tenant_id, election_event_id, election_id, contest_id, area_id, channel
    ORDER BY created_at, id
  ) AS rn
  FROM "sequent_backend"."tally_sheet"
)
UPDATE "sequent_backend"."tally_sheet" t
SET "version" = ranked.rn
FROM ranked WHERE t.id = ranked.id;

CREATE UNIQUE INDEX "tally_sheet_uniq_version" on
  "sequent_backend"."tally_sheet" using btree ("tenant_id", "election_event_id", "election_id", "contest_id", "area_id", "channel", "version")
  WHERE "deleted_at" IS NULL;
