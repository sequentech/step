alter table "sequent_backend"."cast_vote"
  add constraint "cast_vote_tenant_id_fkey"
  foreign key ("tenant_id")
  references "sequent_backend"."tenant"
  ("id") on update restrict on delete restrict;
