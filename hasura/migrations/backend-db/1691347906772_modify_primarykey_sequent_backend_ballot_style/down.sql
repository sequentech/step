alter table "sequent_backend"."ballot_style" drop constraint "ballot_style_pkey";
alter table "sequent_backend"."ballot_style"
    add constraint "ballot_style_pkey"
    primary key ("id", "election_id", "tenant_id");
