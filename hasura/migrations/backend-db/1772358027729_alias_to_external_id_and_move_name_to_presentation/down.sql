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