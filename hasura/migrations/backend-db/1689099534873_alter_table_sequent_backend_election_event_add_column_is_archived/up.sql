alter table "sequent_backend"."election_event" add column "is_archived" boolean
 not null default 'false';
