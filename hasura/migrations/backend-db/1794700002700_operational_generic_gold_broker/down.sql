-- Never roll back the old-claim interlock while production grants exist.
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.operational_gold_device_reservation) THEN
        RAISE EXCEPTION 'cannot remove production HSM claim protection while grants exist';
    END IF;
END;
$$;
DROP TRIGGER operational_gold_require_document ON sequent_backend.operational_signing_intent;
DROP FUNCTION sequent_backend.operational_gold_require_document();
DROP TRIGGER operational_gold_require_ca ON sequent_backend.operational_signing_device_grant;
DROP FUNCTION sequent_backend.operational_gold_require_ca();
DROP FUNCTION sequent_backend.operational_gold_claim(uuid,uuid,uuid,text,uuid,uuid,text,text);
DROP FUNCTION sequent_backend.operational_gold_get_ca(uuid,uuid,uuid,text,uuid,uuid,text);
DROP FUNCTION sequent_backend.operational_gold_reserve(sequent_backend.operational_signing_binding,bigint,timestamptz);
DROP FUNCTION sequent_backend.operational_gold_store_work(sequent_backend.operational_signing_binding,bytea);
DROP FUNCTION sequent_backend.operational_gold_link_work(sequent_backend.operational_signing_binding,uuid,bytea);
DROP FUNCTION sequent_backend.operational_gold_load_work(uuid,uuid,uuid,text,text,uuid);
DROP FUNCTION sequent_backend.operational_gold_resume_payload(uuid,uuid,uuid,text,text);
DROP FUNCTION sequent_backend.operational_gold_reconcile(uuid,uuid,uuid,text,text);
DROP FUNCTION sequent_backend.operational_gold_status(uuid,uuid,uuid,text,uuid);
DROP FUNCTION sequent_backend.operational_gold_load_attempt(uuid,uuid,uuid,text,uuid);
DROP FUNCTION sequent_backend.operational_gold_take_callback(sequent_backend.operational_signing_binding);
DROP FUNCTION sequent_backend.operational_gold_prepare(sequent_backend.operational_signing_binding);
DROP FUNCTION sequent_backend.operational_gold_scrub_expired_work(integer);
DROP FUNCTION sequent_backend.operational_gold_current_key(sequent_backend.operational_signing_binding);
DROP TABLE sequent_backend.operational_gold_device_reservation;
DROP TABLE sequent_backend.operational_gold_work;
DROP TABLE sequent_backend.operational_gold_callback;
REVOKE USAGE ON TYPE sequent_backend.operational_signing_binding FROM sequent_os_gold_broker;
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_gold_broker, sequent_os_gold_ca, sequent_os_gold_gc;
-- Role intentionally retained so this migration does not remove an operator membership.
