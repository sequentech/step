alter table "sequent_backend"."communication_template" alter column "election_event_id" drop not null;
alter table "sequent_backend"."communication_template" add column "election_event_id" uuid;
