alter table "sequent_backend"."trustee" alter column "is_protocol_manager" set default false;
alter table "sequent_backend"."trustee" alter column "is_protocol_manager" drop not null;
alter table "sequent_backend"."trustee" add column "is_protocol_manager" bool;
