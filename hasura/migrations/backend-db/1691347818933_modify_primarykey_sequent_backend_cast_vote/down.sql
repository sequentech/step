alter table "sequent_backend"."cast_vote" drop constraint "cast_vote_pkey";
alter table "sequent_backend"."cast_vote"
    add constraint "cast_vote_pkey"
    primary key ("id");
