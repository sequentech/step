alter table "sequent_backend"."ballot_publication" alter column "is_election_event" drop not null;
alter table "sequent_backend"."ballot_publication" add column "is_election_event" bool;
