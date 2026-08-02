-- ===============================
-- candidate
-- ===============================
do $$
begin
  if not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='candidate' and column_name='name'
  ) then
    alter table "sequent_backend"."candidate" add column "name" text;
  end if;
end $$;

update "sequent_backend"."candidate"
set "name" = coalesce(
  nullif("presentation" #>> '{i18n,en,name}', ''),
  '-'
)
where "name" is null;

do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='candidate' and column_name='external_id'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='candidate' and column_name='alias'
  ) then
    alter table "sequent_backend"."candidate"
    rename column "external_id" to "alias";
  end if;
end $$;


-- ===============================
-- contest
-- ===============================
do $$
begin
  if not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='contest' and column_name='name'
  ) then
    alter table "sequent_backend"."contest" add column "name" text;
  end if;
end $$;

update "sequent_backend"."contest"
set "name" = coalesce(
  nullif("presentation" #>> '{i18n,en,name}', ''),
  '-'
)
where "name" is null;

do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='contest' and column_name='external_id'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='contest' and column_name='alias'
  ) then
    alter table "sequent_backend"."contest"
    rename column "external_id" to "alias";
  end if;
end $$;


-- ===============================
-- election
-- ===============================
do $$
begin
  if not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election' and column_name='name'
  ) then
    alter table "sequent_backend"."election" add column "name" text;
  end if;
end $$;

update "sequent_backend"."election"
set "name" = coalesce(
  nullif("presentation" #>> '{i18n,en,name}', ''),
  '-'
)
where "name" is null;

alter table "sequent_backend"."election"
alter column "name" set not null;

do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election' and column_name='external_id'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election' and column_name='alias'
  ) then
    alter table "sequent_backend"."election"
    rename column "external_id" to "alias";
  end if;
end $$;


-- ===============================
-- election_event
-- ===============================
do $$
begin
  if not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election_event' and column_name='name'
  ) then
    alter table "sequent_backend"."election_event" add column "name" text;
  end if;
end $$;

update "sequent_backend"."election_event"
set "name" = coalesce(
  nullif("presentation" #>> '{i18n,en,name}', ''),
  '-'
)
where "name" is null;

alter table "sequent_backend"."election_event"
alter column "name" set not null;

do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election_event' and column_name='external_id'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election_event' and column_name='alias'
  ) then
    alter table "sequent_backend"."election_event"
    rename column "external_id" to "alias";
  end if;
end $$;