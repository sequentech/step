DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_uniq_version";
CREATE UNIQUE INDEX "tally_sheet_uniq_version" on
  "sequent_backend"."tally_sheet" using btree ("tenant_id", "election_event_id", "election_id", "contest_id", "area_id", "channel", "version");

DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_import_item_event_idx";
DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_import_item_ballot_box_idx";
DROP TABLE IF EXISTS "sequent_backend"."tally_sheet_import_item";

ALTER TABLE "sequent_backend"."tally_sheet"
    DROP CONSTRAINT IF EXISTS "tally_sheet_import_id_fkey";

ALTER TABLE "sequent_backend"."tally_sheet"
    DROP COLUMN IF EXISTS "import_id";

DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_import_document_idx";
DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_import_event_idx";
DROP TABLE IF EXISTS "sequent_backend"."tally_sheet_import";
