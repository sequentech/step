alter table "sequent_backend"."results_election" alter column "blank_votes" drop not null;
alter table "sequent_backend"."results_election" add column "blank_votes" int4;
