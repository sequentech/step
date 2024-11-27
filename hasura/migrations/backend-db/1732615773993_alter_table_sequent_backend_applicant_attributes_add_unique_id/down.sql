alter table "sequent_backend"."applicant_attributes" drop constraint "applicant_attributes_id_key";
alter table "sequent_backend"."applicant_attributes" add constraint "applicant_attributes_applicant_id_key" unique ("applicant_id");
