alter table "sequent_backend"."ballot_style"
  add constraint "ballot_style_election_event_id_tenant_id_ballot_publication_"
  foreign key ("election_event_id", "tenant_id", "ballot_publication_id")
  references "sequent_backend"."ballot_publication"
  ("election_event_id", "tenant_id", "id") on update restrict on delete restrict;
