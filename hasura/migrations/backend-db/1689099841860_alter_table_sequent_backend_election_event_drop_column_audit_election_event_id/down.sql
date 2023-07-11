alter table "sequent_backend"."election_event" alter column "audit_election_event_id" drop not null;
alter table "sequent_backend"."election_event" add column "audit_election_event_id" int4;
