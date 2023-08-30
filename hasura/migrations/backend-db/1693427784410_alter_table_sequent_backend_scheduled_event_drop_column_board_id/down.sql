alter table "sequent_backend"."scheduled_event" alter column "board_id" drop not null;
alter table "sequent_backend"."scheduled_event" add column "board_id" int4;
