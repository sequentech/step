-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Apply transactionally. Preserve operator records and all ceremony evidence.
LOCK TABLE felix3_initialize_setup.permit, felix3_initialize_setup.attempt IN ACCESS EXCLUSIVE MODE NOWAIT;
DO $$
BEGIN
    IF EXISTS (SELECT FROM felix3_initialize_setup.permit)
       OR EXISTS (SELECT FROM felix3_initialize_setup.attempt) THEN
        RAISE EXCEPTION 'local enrollment permits or attempts require operator reconciliation';
    END IF;
END;
$$;

DROP FUNCTION
    felix3_initialize_setup.status(felix3_initialize_setup.binding),
    felix3_initialize_setup.complete(felix3_initialize_setup.binding,uuid,uuid,text,text,text),
    felix3_initialize_setup.claim(felix3_initialize_setup.binding,uuid),
    felix3_initialize_setup.release(felix3_initialize_setup.binding,uuid),
    felix3_initialize_setup.confirm_gold(felix3_initialize_setup.binding,uuid,uuid,text,text,text,bigint,bigint,bigint),
    felix3_initialize_setup.gold_challenge(felix3_initialize_setup.binding,uuid,text,boolean),
    felix3_initialize_setup.begin_setup(felix3_initialize_setup.binding,uuid,text),
    felix3_initialize_setup.eligible(felix3_initialize_setup.binding,boolean),
    felix3_initialize_setup.payload_matches(felix3_initialize_setup.binding,jsonb,text,timestamptz),
    felix3_initialize_setup.fresh_gold(bigint,bigint,bigint,bigint,bigint),
    felix3_initialize_setup.principal(name);
DROP TABLE felix3_initialize_setup.attempt, felix3_initialize_setup.permit;
DROP TYPE felix3_initialize_setup.binding;
DROP SCHEMA felix3_initialize_setup RESTRICT;
