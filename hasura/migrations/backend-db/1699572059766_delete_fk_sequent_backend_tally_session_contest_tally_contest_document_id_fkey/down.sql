alter table "sequent_backend"."tally_session_contest"
  add constraint "tally_contest_document_id_fkey"
  foreign key ("document_id")
  references "sequent_backend"."document"
  ("id") on update restrict on delete restrict;
