alter table "sequent_backend"."results_election" alter column "explicit_invalid_votes" drop not null;
alter table "sequent_backend"."results_election" add column "explicit_invalid_votes" int4;
