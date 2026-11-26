-- SPDX-License-Identifier: AGPL-3.0-only
-- Run transactionally. Retain the officer's decisions and completed evidence.
LOCK TABLE felix3_initialize_attachment.decision IN ACCESS EXCLUSIVE MODE NOWAIT;
DO $$
BEGIN
    IF EXISTS (SELECT FROM felix3_initialize_attachment.decision) THEN
        RAISE EXCEPTION 'local certificate decisions require operator reconciliation';
    END IF;
END;
$$;
DROP FUNCTION
    felix3_initialize_attachment.finish(uuid,uuid),
    felix3_initialize_attachment.admit(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz,jsonb,text,timestamptz,name,integer,xid8),
    felix3_initialize_attachment.inspect(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz,jsonb),
    felix3_initialize_attachment.review_csr(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz),
    felix3_initialize_attachment.trust(felix3_initialize_setup.binding,jsonb),
    felix3_initialize_attachment.certificate_hash(text),
    felix3_initialize_attachment.generation(felix3_initialize_setup.binding,uuid,uuid,text[],text[]),
    felix3_initialize_attachment.registry_writer(name),
    felix3_initialize_attachment.officer_service(),
    felix3_initialize_attachment.post_access(felix3_initialize_setup.binding,text[],text[]),
    felix3_initialize_attachment.lock_registry();
DROP TABLE felix3_initialize_attachment.decision;
DROP SCHEMA felix3_initialize_attachment RESTRICT;
