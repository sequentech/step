alter table "sequent_backend"."tally_sheet" add column "published_at" timestamptz
 null;

alter table "sequent_backend"."tally_sheet" drop column "reviewed_at";

alter table "sequent_backend"."tally_sheet" add column "published_by_user_id" text
 null;

alter table "sequent_backend"."tally_sheet" drop column "reviewed_by_user_id";

alter table "sequent_backend"."tally_sheet" drop column "status";

alter table "sequent_backend"."tally_sheet" drop column "version";

DROP INDEX IF EXISTS "sequent_backend"."tally_sheet_uniq_channel";
