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
