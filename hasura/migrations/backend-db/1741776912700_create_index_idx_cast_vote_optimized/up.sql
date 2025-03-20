CREATE  INDEX "idx_cast_vote_optimized" on
  "sequent_backend"."cast_vote" using btree ("tenant_id", "election_event_id", "area_id", "status", "election_id", "voter_id_string", "created_at");
