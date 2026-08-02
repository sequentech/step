-- ===============================
-- candidate
-- ===============================
do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='candidate' and column_name='alias'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='candidate' and column_name='external_id'
  ) then
    alter table "sequent_backend"."candidate"
    rename column "alias" to "external_id";
  end if;

  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='candidate' and column_name='name'
  ) then
    update "sequent_backend"."candidate"
    set "presentation" =
      jsonb_set(
        coalesce("presentation", '{}'::jsonb),
        '{i18n,en,name}',
        to_jsonb("name"),
        true
      )
    where "name" is not null;

    alter table "sequent_backend"."candidate"
    drop column "name";
  end if;
end $$;

-- ===============================
-- contest
-- ===============================
do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='contest' and column_name='alias'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='contest' and column_name='external_id'
  ) then
    alter table "sequent_backend"."contest"
    rename column "alias" to "external_id";
  end if;

  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='contest' and column_name='name'
  ) then
    update "sequent_backend"."contest"
    set "presentation" =
      jsonb_set(
        coalesce("presentation", '{}'::jsonb),
        '{i18n,en,name}',
        to_jsonb("name"),
        true
      )
    where "name" is not null;

    alter table "sequent_backend"."contest"
    drop column "name";
  end if;
end $$;

-- ===============================
-- election
-- ===============================
do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election' and column_name='alias'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election' and column_name='external_id'
  ) then
    alter table "sequent_backend"."election"
    rename column "alias" to "external_id";
  end if;

  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election' and column_name='name'
  ) then
    update "sequent_backend"."election"
    set "presentation" =
      jsonb_set(
        coalesce("presentation", '{}'::jsonb),
        '{i18n,en,name}',
        to_jsonb("name"),
        true
      )
    where "name" is not null;

    alter table "sequent_backend"."election"
    drop column "name";
  end if;
end $$;

-- ===============================
-- election_event
-- ===============================
do $$
begin
  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election_event' and column_name='alias'
  ) and not exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election_event' and column_name='external_id'
  ) then
    alter table "sequent_backend"."election_event"
    rename column "alias" to "external_id";
  end if;

  if exists (
    select 1 from information_schema.columns
    where table_schema='sequent_backend' and table_name='election_event' and column_name='name'
  ) then
    update "sequent_backend"."election_event"
    set "presentation" =
      jsonb_set(
        coalesce("presentation", '{}'::jsonb),
        '{i18n,en,name}',
        to_jsonb("name"),
        true
      )
    where "name" is not null;

    alter table "sequent_backend"."election_event"
    drop column "name";
  end if;
end $$;