CREATE  INDEX "idx_applications_tenant_election_created" on
  "sequent_backend"."applications" using btree ("tenant_id", "election_event_id", "created_at");
