-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Server-side unwrap of an encrypted configuration package that carries
-- messaging credentials (harvest configuration_package_intake).
--
-- uploaded_by: the verified token's user ID (hasura_claims.user_id, the ID
--   the audit actor and Apply's task carry) that requested this protected
--   staging row. Only that user may unwrap it, and only that user's Preview
--   and Apply may use the credentials staged with it.
-- unwrapped_id: names a server-only sibling object holding the unwrapped
--   signed ZIP. It is never presigned, so the browser's upload URL can't
--   overwrite it. Set at most once, in the transaction that stages the
--   credentials in the vault; readers use it in place of delivery.zip.
-- credentials_expires_at: when the staged credentials stop being accepted
--   and may be swept from the vault. Cleared once Apply consumes them.
--
-- All nullable, no backfill: existing rows and ordinary deliveries (unwrapped
-- in the browser, no credentials) behave exactly as before. A row without
-- uploaded_by can't be unwrapped server side. Still not tracked by Hasura.
--
-- Hasura records numeric versions, not directory names. A previous
-- development branch used 1794700001600 for this change. The canonical
-- backend-db migration at 1600 is cast_vote_seal_key. Check the schema,
-- never the metadata number, so a mismatched 1600 cannot skip ballot
-- sealing's policy and still start this release.
LOCK TABLE sequent_backend.configuration_package_staging IN ACCESS EXCLUSIVE MODE;
DO $$
DECLARE
    v_stage constant oid := 'sequent_backend.configuration_package_staging'::regclass;
    v_expiry_index oid := to_regclass('sequent_backend.configuration_package_staging_credentials_expiry');
    v_column_count integer;
    v_columns_match boolean;
    v_constraints_match boolean;
BEGIN
    IF to_regclass('sequent_backend.cast_vote_sealing_required') IS NULL
       OR to_regclass('sequent_backend.cast_vote_seal_key') IS NULL
       OR to_regprocedure('sequent_backend.cast_vote_seal_state(uuid,uuid)') IS NULL
       OR to_regprocedure('sequent_backend.cast_vote_seal_require(uuid,uuid)') IS NULL
       OR to_regprocedure('sequent_backend.cast_vote_seal_publish(uuid,uuid,text,integer,bytea)') IS NULL
       OR (SELECT count(*) FROM pg_catalog.pg_trigger
           WHERE tgname = 'cast_vote_seal_voting_status_lock' AND NOT tgisinternal
             AND tgenabled IN ('O', 'A')
             AND tgfoid = to_regprocedure('sequent_backend.cast_vote_seal_voting_status_lock()')
             AND tgrelid IN ('sequent_backend.election'::regclass, 'sequent_backend.election_event'::regclass)) <> 2
    THEN
        RAISE EXCEPTION 'Ambiguous backend-db version 1794700001600: canonical cast-vote sealing migration is absent or incomplete'
            USING HINT = 'Stop the upgrade; compare the actual schema against 1794700001600_cast_vote_seal_key. See the migration recovery notes in packages/hasura/README.md.';
    END IF;

    SELECT count(a.attnum), coalesce(bool_and(a.atttypid = expected.data_type
        AND NOT a.attnotnull AND NOT a.atthasdef AND a.attidentity = '' AND a.attgenerated = ''), false)
    INTO v_column_count, v_columns_match
    FROM (VALUES ('uploaded_by', 'text'::regtype), ('unwrapped_id', 'uuid'::regtype),
                 ('credentials_expires_at', 'timestamptz'::regtype)) AS expected(name, data_type)
    JOIN pg_catalog.pg_attribute AS a
      ON a.attrelid = v_stage AND a.attname = expected.name AND NOT a.attisdropped;

    IF v_column_count <> 0 OR v_expiry_index IS NOT NULL THEN
        -- On a reconciled prior installation, accept only the exact prior
        -- security checks and index. Never reconstruct partial staging
        -- state or update an uploader, object pointer, or expiry.
        SELECT count(*) = 3 INTO v_constraints_match
        FROM pg_catalog.pg_constraint AS c
        WHERE c.conrelid = v_stage AND c.convalidated AND NOT c.condeferrable
          AND ((c.conname = 'configuration_package_staging_credentials_need_unwrap'
                AND c.contype = 'c'
                AND regexp_replace(pg_get_expr(c.conbin, c.conrelid), '[()[:space:]]', '', 'g')
                    = 'credentials_expires_atISNULLORunwrapped_idISNOTNULL')
            OR (c.conname = 'configuration_package_staging_unwrap_needs_uploader'
                AND c.contype = 'c'
                AND regexp_replace(pg_get_expr(c.conbin, c.conrelid), '[()[:space:]]', '', 'g')
                    = 'unwrapped_idISNULLORuploaded_byISNOTNULL')
            OR (c.conname = 'configuration_package_staging_unwrapped_id_key'
                AND c.contype = 'u'
                AND c.conkey = ARRAY[(SELECT attnum FROM pg_catalog.pg_attribute
                                      WHERE attrelid = v_stage AND attname = 'unwrapped_id' AND NOT attisdropped)]
                AND EXISTS (SELECT FROM pg_catalog.pg_index AS u
                            WHERE u.indexrelid = c.conindid AND u.indisvalid AND u.indisready)));

        IF v_column_count <> 3 OR NOT v_columns_match OR NOT v_constraints_match
           OR NOT EXISTS (SELECT FROM pg_catalog.pg_index AS i
                          JOIN pg_catalog.pg_class AS idx ON idx.oid = i.indexrelid
                          JOIN pg_catalog.pg_am AS am ON am.oid = idx.relam
                          WHERE i.indexrelid = v_expiry_index AND i.indrelid = v_stage
                            AND am.amname = 'btree' AND NOT i.indisunique AND i.indisvalid AND i.indisready
                            AND i.indnatts = 2 AND i.indnkeyatts = 2
                            AND pg_get_indexdef(i.indexrelid, 1, true) = 'tenant_id'
                            AND pg_get_indexdef(i.indexrelid, 2, true) = 'credentials_expires_at'
                            AND regexp_replace(pg_get_expr(i.indpred, i.indrelid), '[()[:space:]]', '', 'g')
                                = 'credentials_expires_atISNOTNULL')
        THEN
            RAISE EXCEPTION 'Configuration package credential intake has an incomplete or unexpected staging schema'
                USING HINT = 'Preserve existing staging and verify the prior migration on a snapshot; do not edit the Hasura migration catalogue.';
        END IF;
        RETURN;
    END IF;

    ALTER TABLE sequent_backend.configuration_package_staging
        ADD COLUMN uploaded_by text,
        ADD COLUMN unwrapped_id uuid UNIQUE,
        ADD COLUMN credentials_expires_at timestamptz,
        ADD CONSTRAINT configuration_package_staging_credentials_need_unwrap
            CHECK (credentials_expires_at IS NULL OR unwrapped_id IS NOT NULL),
        ADD CONSTRAINT configuration_package_staging_unwrap_needs_uploader
            CHECK (unwrapped_id IS NULL OR uploaded_by IS NOT NULL);

    CREATE INDEX configuration_package_staging_credentials_expiry
        ON sequent_backend.configuration_package_staging (tenant_id, credentials_expires_at)
        WHERE credentials_expires_at IS NOT NULL;
END $$;
