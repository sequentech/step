alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_contest_tally_id_tenant_id_election_event_id_fkey"
  foreign key (election_event_id, tally_id, tenant_id)
  references "sequent_backend"."tally_session"
  (election_event_id, id, tenant_id) on update restrict on delete restrict;
alter table "sequent_backend"."tally_session_contest" alter column "tally_id" drop not null;
alter table "sequent_backend"."tally_session_contest" add column "tally_id" uuid;
