alter table "sequent_backend"."communication_template"
  add constraint "communication_template_election_event_id_fkey"
  foreign key ("election_event_id")
  references "sequent_backend"."election_event"
  ("id") on update restrict on delete restrict;
