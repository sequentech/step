alter table "sequent_backend"."election_event"
  add constraint "election_event_audit_election_event_id_fkey"
  foreign key ("audit_election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;
