CREATE TABLE "sequent_backend"."lock" ("key" text NOT NULL, "value" text NOT NULL, "expiry_date" timestamptz, PRIMARY KEY ("key") , UNIQUE ("key"));
