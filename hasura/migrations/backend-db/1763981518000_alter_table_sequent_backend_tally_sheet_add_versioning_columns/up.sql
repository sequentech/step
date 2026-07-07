
alter table "sequent_backend"."tally_sheet" add column "reviewed_at" timestamptz
 null;

alter table "sequent_backend"."tally_sheet" add column "reviewed_by_user_id" text
 null;

UPDATE "sequent_backend"."tally_sheet" SET "reviewed_at" = "published_at", "reviewed_by_user_id" = "published_by_user_id" WHERE "published_at" IS NOT NULL;

alter table "sequent_backend"."tally_sheet" add column "status" text
 null;

UPDATE "sequent_backend"."tally_sheet" SET "status" = CASE WHEN "published_at" IS NOT NULL THEN 'APPROVED' ELSE 'PENDING' END;

alter table "sequent_backend"."tally_sheet" alter column "status" set not null;

alter table "sequent_backend"."tally_sheet" alter column "status" set default 'PENDING';

alter table "sequent_backend"."tally_sheet" add column "version" integer
 null;

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

alter table "sequent_backend"."tally_sheet" alter column "version" set not null;

alter table "sequent_backend"."tally_sheet" drop column if exists "published_at";

alter table "sequent_backend"."tally_sheet" drop column if exists "published_by_user_id";

CREATE UNIQUE INDEX "tally_sheet_uniq_version" on
  "sequent_backend"."tally_sheet" using btree ("tenant_id", "election_event_id", "election_id", "contest_id", "area_id", "channel", "version")
  WHERE "deleted_at" IS NULL;
