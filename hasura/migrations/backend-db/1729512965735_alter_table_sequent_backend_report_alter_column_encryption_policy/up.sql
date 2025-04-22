alter table "sequent_backend"."report" alter column "encryption_policy" set default 'unencrypted';
alter table "sequent_backend"."report" alter column "encryption_policy" drop not null;
