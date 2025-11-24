
alter table "sequent_backend"."tally_sheet" drop column "published_at";

alter table "sequent_backend"."tally_sheet" add column "reviewed_at" timestamptz
 null;

alter table "sequent_backend"."tally_sheet" drop column "published_by_user_id";

alter table "sequent_backend"."tally_sheet" add column "reviewed_by_user_id" text
 null;

alter table "sequent_backend"."tally_sheet" add column "status" text
 null;

alter table "sequent_backend"."tally_sheet" alter column "status" set not null;

alter table "sequent_backend"."tally_sheet" add column "version" integer
 null;
alter table "sequent_backend"."tally_sheet" alter column "version" set not null;

CREATE UNIQUE INDEX "tally_sheet_uniq_version" on
  "sequent_backend"."tally_sheet" using btree ("tenant_id", "election_event_id", "election_id", "contest_id", "area_id", "channel", "version");
