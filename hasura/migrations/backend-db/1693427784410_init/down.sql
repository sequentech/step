
alter table "sequent_backend"."scheduled_event" alter column "board_id" drop not null;
alter table "sequent_backend"."scheduled_event" add column "board_id" int4;

DROP TABLE "sequent_backend"."trustee";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."scheduled_event" add column "board_id" integer
--  null;

ALTER TABLE "sequent_backend"."event_execution" ALTER COLUMN "id" drop default;

alter table "sequent_backend"."scheduled_event" rename column "created_by" to "created_nby";

DROP TABLE "sequent_backend"."event_execution";

DROP TABLE "sequent_backend"."scheduled_event";

alter table "sequent_backend"."document" rename to "election_document";

DROP TABLE "sequent_backend"."election_document";

DELETE FROM "sequent_backend"."ballot_style" WHERE "election_event_id" = '33f18502-a67c-4853-8333-a58630663559' AND "id" = '6e5bbff4-0fb8-4971-a808-37de49573f6a' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

DELETE FROM "sequent_backend"."election" WHERE "id" = 'f2f1065e-b784-46d1-b81a-c71bfeb9ad55' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559';

DELETE FROM "sequent_backend"."area" WHERE "id" = '2f312a36-f39c-46e4-9670-1d1ce4625745' AND "election_event_id" = '33f18502-a67c-4853-8333-a58630663559' AND "tenant_id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

DELETE FROM "sequent_backend"."election_event" WHERE "id" = '33f18502-a67c-4853-8333-a58630663559';

DELETE FROM "sequent_backend"."tenant" WHERE "id" = '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_contest_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_pkey";
alter table "sequent_backend"."contest"
    add constraint "contest_pkey"
    primary key ("election_id", "tenant_id", "id");

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_contest_id_election_event_id_fkey"
  foreign key ("election_event_id", "contest_id", "tenant_id")
  references "sequent_backend"."contest"
  ("election_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_tenant_id_election_event_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_election_event_id_area_id_fkey",
  add constraint "area_contest_election_event_id_tenant_id_area_id_fkey"
  foreign key ("election_event_id", "area_id", "tenant_id")
  references "sequent_backend"."area"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."contest" drop constraint "contest_tenant_id_fkey",
  add constraint "contest_id_fkey"
  foreign key ("id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" rename column "content" to "cast_ballot_eml";

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_pkey";
alter table "sequent_backend"."election"
    add constraint "election_pkey"
    primary key ("tenant_id", "id", "election_event_id");

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_id", "tenant_id", "election_event_id")
  references "sequent_backend"."election"
  ("id", "tenant_id", "election_event_id") on update restrict on delete restrict;

alter table "sequent_backend"."contest"
  add constraint "contest_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "election_id", "tenant_id")
  references "sequent_backend"."election"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_event_id", "election_id", "tenant_id")
  references "sequent_backend"."election"
  ("election_event_id", "id", "tenant_id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_event_id_tenant_id_election_id_fkey"
  foreign key ("election_id", "tenant_id", "election_event_id")
  references "sequent_backend"."election"
  ("id", "tenant_id", "election_event_id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_pkey";
alter table "sequent_backend"."cast_vote"
    add constraint "cast_vote_pkey"
    primary key ("election_event_id", "id", "tenant_id");

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_pkey";
alter table "sequent_backend"."ballot_style"
    add constraint "ballot_style_pkey"
    primary key ("id", "election_id", "tenant_id");

alter table "sequent_backend"."candidate" drop constraint "candidate_pkey";
alter table "sequent_backend"."candidate"
    add constraint "candidate_pkey"
    primary key ("election_event_id", "tenant_id");

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_pkey";
alter table "sequent_backend"."cast_vote"
    add constraint "cast_vote_pkey"
    primary key ("id");

alter table "sequent_backend"."candidate" drop constraint "candidate_pkey";
alter table "sequent_backend"."candidate"
    add constraint "candidate_pkey"
    primary key ("id");

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_pkey";
alter table "sequent_backend"."ballot_style"
    add constraint "ballot_style_pkey"
    primary key ("id");

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_tenant_id_election_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_pkey";
alter table "sequent_backend"."election"
    add constraint "election_pkey"
    primary key ("id");

alter table "sequent_backend"."contest"
  add constraint "contest_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."election_result"
  add constraint "election_result_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_election_id_fkey"
  foreign key ("election_id")
  references "sequent_backend"."election"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_contest_id_election_event_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_pkey";
alter table "sequent_backend"."contest"
    add constraint "contest_pkey"
    primary key ("id");

alter table "sequent_backend"."candidate"
  add constraint "candidate_contest_id_fkey"
  foreign key ("contest_id")
  references "sequent_backend"."contest"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_contest_id_fkey"
  foreign key ("contest_id")
  references "sequent_backend"."contest"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_tenant_id_area_id_fkey";

alter table "sequent_backend"."area" drop constraint "area_pkey";
alter table "sequent_backend"."area"
    add constraint "area_pkey"
    primary key ("id");

alter table "sequent_backend"."election_result"
  add constraint "election_result_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

alter table "sequent_backend"."area_contest"
  add constraint "area_contest_area_id_fkey"
  foreign key ("area_id")
  references "sequent_backend"."area"
  ("id") on update restrict on delete restrict;

ALTER TABLE "sequent_backend"."election_result" ALTER COLUMN "result_eml" TYPE bytea;

ALTER TABLE "sequent_backend"."cast_vote" ALTER COLUMN "cast_ballot_eml" TYPE bytea;

ALTER TABLE "sequent_backend"."ballot_style" ALTER COLUMN "ballot_eml" TYPE bytea;

ALTER TABLE "sequent_backend"."election" ALTER COLUMN "eml" TYPE bytea;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "conditions" jsonb
--  null;

alter table "sequent_backend"."contest" drop constraint "contest_election_event_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_election_event_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_election_event_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_event_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "election_event_id" uuid
--  null;

alter table "sequent_backend"."election_result" drop constraint "election_result_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "voter_id_string" varchar
--  null;

alter table "sequent_backend"."election_result" drop constraint "election_result_area_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_election_id_fkey";

alter table "sequent_backend"."election_result" drop constraint "election_result_tenant_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_area_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_election_id_fkey";

alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_tenant_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_area_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_election_id_fkey";

alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_tenant_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_contest_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_area_id_fkey";

alter table "sequent_backend"."area_contest" drop constraint "area_contest_tenant_id_fkey";

alter table "sequent_backend"."area" drop constraint "area_election_event_id_fkey";

alter table "sequent_backend"."area" drop constraint "area_tenant_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_contest_id_fkey";

alter table "sequent_backend"."candidate" drop constraint "candidate_tenant_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_election_id_fkey";

alter table "sequent_backend"."contest" drop constraint "contest_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_election_event_id_fkey";

alter table "sequent_backend"."election" drop constraint "election_tenant_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "statistics" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "result_eml_signature" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "result_eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "cast_ballot_signature" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "cast_ballot_eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "status" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "ballot_signature" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "ballot_eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_result" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."cast_vote" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."ballot_style" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "created_at" timestamptz
--  null default now();

alter table "sequent_backend"."area_contest" alter column "created_at" drop not null;
alter table "sequent_backend"."area_contest" add column "created_at" jsonb;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "created_at" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "area_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "contest_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area_contest" add column "tenant_id" uuid
--  null;

alter table "sequent_backend"."area_contest" rename to "area_context";

DROP TABLE "sequent_backend"."area_context";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "type" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "name" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."area" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "is_public" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "type" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "name" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "contest_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."candidate" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "tally_configuration" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "is_encrypted" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "counting_algorithm" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "voting_type" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "max_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "min_votes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "name" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "is_active" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "is_acclaimed" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "created_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "election_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."contest" add column "tenant_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "spoil_ballot_option" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "is_consolidated_ballot_encoding" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "num_allowed_revotes" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "eml" bytea
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "status" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "dates" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "name" varchar
--  not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "annotations" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "labels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "last_updated_at" timestamptz
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "created_at" timestamptz
--  null default now();

alter table "sequent_backend"."election" alter column "created_at" set default now();
alter table "sequent_backend"."election" alter column "created_at" drop not null;
alter table "sequent_backend"."election" add column "created_at" timestamp;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "created_at" Timestamp
--  null default now();

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "election_event_id" uuid
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election" add column "tenant_id" uuid
--  null;

alter table "sequent_backend"."election_event" drop constraint "election_event_audit_election_event_id_fkey";

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "audit_election_event_id" uuid
--  null;

alter table "sequent_backend"."election_event" alter column "audit_election_event_id" drop not null;
alter table "sequent_backend"."election_event" add column "audit_election_event_id" int4;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "audit_election_event_id" integer
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "is_audit" boolean
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "encryption_protocol" varchar
--  not null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "user_boards" varchar
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "status" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "dates" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "voting_channels" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "is_archived" boolean
--  not null default 'false';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "bulletin_board_reference" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "presentation" jsonb
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "description" text
--  null;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "name" varchar
--  not null;

DROP TABLE "sequent_backend"."election_result";

DROP TABLE "sequent_backend"."cast_vote";

DROP TABLE "sequent_backend"."ballot_style";

ALTER TABLE "sequent_backend"."area" ALTER COLUMN "id" drop default;

DROP TABLE "sequent_backend"."area";

ALTER TABLE "sequent_backend"."candidate" ALTER COLUMN "id" drop default;

DROP TABLE "sequent_backend"."candidate";

ALTER TABLE "sequent_backend"."contest" ALTER COLUMN "id" drop default;

DROP TABLE "sequent_backend"."contest";

DROP TABLE "sequent_backend"."election";

alter table "sequent_backend"."election_event" alter column "is_active" set default false;
alter table "sequent_backend"."election_event" alter column "is_active" drop not null;
alter table "sequent_backend"."election_event" add column "is_active" bool;

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."tenant" add column "is_active" boolean
--  not null default 'false';

-- Could not auto-generate a down migration.
-- Please write an appropriate down migration for the SQL below:
-- alter table "sequent_backend"."election_event" add column "is_active" boolean
--  not null default 'false';

alter table "sequent_backend"."election_event" rename to "event";


DROP INDEX IF EXISTS "sequent_backend"."tenant_labels";

DROP INDEX IF EXISTS "sequent_backend"."event_labels";

DROP TABLE "sequent_backend"."event";

DROP TABLE "sequent_backend"."tenant";

drop schema "sequent_backend" cascade;
