alter table "sequent_backend"."applicant_attributes" drop constraint "applicant_attributes_pkey";
alter table "sequent_backend"."applicant_attributes"
    add constraint "applicant_attributes_pkey"
    primary key ("id", "applicant_id");
