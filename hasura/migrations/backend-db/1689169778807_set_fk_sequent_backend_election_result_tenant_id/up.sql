alter table "sequent_backend"."election_result"
  add constraint "election_result_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
