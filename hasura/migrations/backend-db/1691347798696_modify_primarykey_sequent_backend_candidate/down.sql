alter table "sequent_backend"."candidate" drop constraint "candidate_pkey";
alter table "sequent_backend"."candidate"
    add constraint "candidate_pkey"
    primary key ("id");
