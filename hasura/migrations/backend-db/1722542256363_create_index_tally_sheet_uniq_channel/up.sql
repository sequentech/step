CREATE UNIQUE INDEX "tally_sheet_uniq_channel" on
  "sequent_backend"."tally_sheet" using btree ("tenant_id", "election_event_id", "election_id", "contest_id", "area_id", "channel");
