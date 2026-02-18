alter table "sequent_backend"."election" add constraint "election_election_event_id_external_id_key" unique ("election_event_id", "external_id");
