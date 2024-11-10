alter table "sequent_backend"."tasks_execution" drop constraint "tasks_execution_pkey";
alter table "sequent_backend"."tasks_execution"
    add constraint "tasks_execution_pkey"
    primary key ("election_event_id", "tenant_id", "id");
