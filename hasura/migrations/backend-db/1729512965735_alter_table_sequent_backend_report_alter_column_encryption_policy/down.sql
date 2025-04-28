alter table "sequent_backend"."report" alter column "encryption_policy" set not null;
alter table "sequent_backend"."report" alter column "encryption_policy" set default 'unencrypted'::character varying;
