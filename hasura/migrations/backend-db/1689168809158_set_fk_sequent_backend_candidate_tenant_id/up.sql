alter table "sequent_backend"."candidate"
  add constraint "candidate_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
