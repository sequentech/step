-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Receiver-only ownership. Names and sender identities are not proof that
-- a signed package owns a tenant account. This table is not exposed by Hasura.
CREATE TABLE sequent_backend.portable_messaging_account_binding (
    tenant_id uuid NOT NULL,
    reference text NOT NULL CHECK (reference ~ '^package:[a-z]([a-z0-9-]{0,46}[a-z0-9])?$'),
    account_id uuid NOT NULL,
    applied_package_sha256 text CHECK (applied_package_sha256 ~ '^[0-9a-f]{64}$'),
    -- Names and replacement timestamps only, as on messaging_account.
    credential_metadata jsonb,
    PRIMARY KEY (tenant_id, reference),
    UNIQUE (tenant_id, account_id),
    FOREIGN KEY (tenant_id, account_id)
        REFERENCES sequent_backend.messaging_account (tenant_id, id)
        ON UPDATE CASCADE ON DELETE CASCADE,
    CHECK ((applied_package_sha256 IS NULL AND credential_metadata IS NULL)
        OR (applied_package_sha256 IS NOT NULL AND credential_metadata IS NOT NULL
            AND jsonb_typeof(credential_metadata) = 'object'))
);

-- Before this migration a signed Apply recorded creation and the exact local
-- UUID together in its own transaction. Reuse is not ownership. Only the
-- receiver's well-formed, unambiguous CREATE receipts for UUIDv8 accounts are
-- accepted. Do not infer ownership from a tenant-edited name, sender, or UUID
-- alone, and do not assume any pre-upgrade credentials are still installed.
WITH evidence AS (
    SELECT DISTINCT a.tenant_id, c.change->>'reference' AS reference, a.id AS account_id
    FROM sequent_backend.audit_log_outbox o
    CROSS JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(o.details#>'{extra,changes}') = 'array'
             THEN o.details#>'{extra,changes}' ELSE '[]'::jsonb END
    ) AS c(change)
    JOIN sequent_backend.messaging_account a ON a.tenant_id = o.tenant_id
      AND a.id::text = (o.details#>'{extra,recipientAccountIds}')->>(c.change->>'reference')
    WHERE o.election_event_id IS NULL
      AND o.statement_kind = 'AdminChange'
      AND o.event_type = 'USER'
      AND o.user_id IS NOT NULL
      AND o.details->>'origin' = 'windmill'
      AND o.details->>'action' = 'import'
      AND o.details#>>'{object,type}' = 'tenant'
      AND o.details#>>'{object,id}' = o.tenant_id::text
      AND o.details#>>'{extra,section}' = 'messaging-accounts'
      AND c.change->>'action' = 'create'
      AND c.change->>'reference' ~ '^package:[a-z]([a-z0-9-]{0,46}[a-z0-9])?$'
      AND substr(a.id::text, 15, 1) = '8'
), bindings AS (
    SELECT *,
           count(*) OVER (PARTITION BY tenant_id, reference) AS same_ref,
           count(*) OVER (PARTITION BY tenant_id, account_id) AS same_id
    FROM evidence
)
INSERT INTO sequent_backend.portable_messaging_account_binding (tenant_id, reference, account_id)
SELECT tenant_id, reference, account_id FROM bindings WHERE same_ref = 1 AND same_id = 1;
