alter table "sequent_backend"."contest" alter column "under_vote_alert" set default false;
alter table "sequent_backend"."contest" alter column "under_vote_alert" drop not null;
alter table "sequent_backend"."contest" add column "under_vote_alert" bool;
