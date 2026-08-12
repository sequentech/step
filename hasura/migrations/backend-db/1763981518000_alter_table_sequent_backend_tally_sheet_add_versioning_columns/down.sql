alter table "sequent_backend"."tally_sheet" add column "published_at" timestamptz
 null;

alter table "sequent_backend"."tally_sheet" add column "published_by_user_id" text
 null;

UPDATE "sequent_backend"."tally_sheet" SET "published_at" = "reviewed_at", "published_by_user_id" = "reviewed_by_user_id" WHERE "status" = 'APPROVED';

alter table "sequent_backend"."tally_sheet" drop column if exists "reviewed_at";

alter table "sequent_backend"."tally_sheet" drop column if exists "reviewed_by_user_id";

alter table "sequent_backend"."tally_sheet" drop column if exists "status";

alter table "sequent_backend"."tally_sheet" drop column if exists "version";

DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_uniq_version";
