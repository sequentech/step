alter table "sequent_backend"."tally_session" alter column "area_ids" drop not null;
alter table "sequent_backend"."tally_session" add column "area_ids" _uuid;
