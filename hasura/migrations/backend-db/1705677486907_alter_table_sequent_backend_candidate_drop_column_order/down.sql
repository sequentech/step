alter table "sequent_backend"."candidate" alter column "order" drop not null;
alter table "sequent_backend"."candidate" add column "order" int4;
