-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Signing quorum for protected actions: per-event rules and certificate
-- checks, signing requests and the approvals that complete them, the staff
-- certificates signers registered, the CRLs of their issuers, the PDF
-- revisions a signed report goes through, and the outbox that carries every
-- signing step to the electoral log.
--
-- Every row belongs to one election event of its own tenant and goes when the
-- event goes. The text lists in the CHECK constraints mirror the kebab-case
-- serde forms of the sequent_core::signing enums; a new variant needs a
-- migration deployed before the code that writes it. Hashes are lowercase hex
-- SHA-256, so that the uniqueness of fingerprints, keys and holders cannot be
-- sidestepped by writing one hash in two ways. User ids are Keycloak ids.
-- The *_name columns hold people's display names as they were when the row
-- was written (first and last name, or the username), for screens that read
-- these tables through Hasura, which can't join Keycloak.
-- Timestamps the database writes use clock_timestamp(), so rows written in one
-- transaction keep the order they were written in.
--
-- Every transaction that writes signing rows first takes the event's advisory
-- lock (windmill::postgres::signing::lock_signing_event), before any row lock:
-- signing steps of an event then commit, and reach the electoral log, in the
-- order they were made.

-- Staff issuers are certificate authorities of their own purpose; every
-- existing row, and every row written without a purpose, signs voters in.
-- One certificate may be both kinds in an event, once each.
ALTER TABLE sequent_backend.certificate_authority
    ADD COLUMN purpose text NOT NULL DEFAULT 'voter-sign-in',
    ADD CONSTRAINT certificate_authority_purpose_known
        CHECK (purpose IN ('voter-sign-in', 'staff-signatures')),
    DROP CONSTRAINT certificate_authority_tenant_id_election_event_id_fingerpri_key,
    ADD CONSTRAINT certificate_authority_one_per_purpose
        UNIQUE (tenant_id, election_event_id, purpose, fingerprint_sha256),
    -- Referenced by the CRLs of staff issuers, which stay in their event.
    ADD CONSTRAINT certificate_authority_in_its_event
        UNIQUE (tenant_id, election_event_id, id);

-- The rule of one action. An event without a row for an action keeps the
-- default rule: no signatures needed. `revision` moves on with every save.
CREATE TABLE sequent_backend.signing_rule (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    action text NOT NULL,
    requirement text NOT NULL,
    signatures integer NOT NULL,
    requester_signing text NOT NULL,
    -- NULL: a request waits without a time limit.
    expires_minutes integer,
    revision bigint NOT NULL,
    updated_by text NOT NULL,
    updated_by_name text,
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT signing_rule_one_per_action
        UNIQUE (tenant_id, election_event_id, action),
    CONSTRAINT signing_rule_action_known CHECK (action IN (
        'initialize-voting', 'open-voting', 'close-voting',
        'generate-election-returns', 'generate-reports', 'transmit-results',
        'approve-voter', 'approve-configuration', 'key-ceremony', 'tally-key'
    )),
    CONSTRAINT signing_rule_requirement_known
        CHECK (requirement IN ('not-required', 'required')),
    CONSTRAINT signing_rule_requester_signing_known
        CHECK (requester_signing IN ('allowed', 'not-allowed')),
    CONSTRAINT signing_rule_needs_a_signature CHECK (signatures >= 1),
    CONSTRAINT signing_rule_expires_later CHECK (expires_minutes > 0),
    CONSTRAINT signing_rule_revision_counts CHECK (revision >= 0),
    CONSTRAINT signing_rule_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- The certificate checks of an event. Without a row the event keeps the
-- defaults: check revocation, refuse without a CRL, register on first use,
-- one Post per certificate.
CREATE TABLE sequent_backend.signing_checks (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    revocation_check text NOT NULL,
    crl_unavailable text NOT NULL,
    registration text NOT NULL,
    post_binding text NOT NULL,
    revision bigint NOT NULL,
    updated_by text NOT NULL,
    updated_by_name text,
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT signing_checks_one_per_event UNIQUE (tenant_id, election_event_id),
    CONSTRAINT signing_checks_revocation_check_known
        CHECK (revocation_check IN ('check', 'dont-check')),
    CONSTRAINT signing_checks_crl_unavailable_known
        CHECK (crl_unavailable IN ('refuse', 'accept-unchecked')),
    CONSTRAINT signing_checks_registration_known
        CHECK (registration IN ('on-first-use', 'security-officer-only')),
    CONSTRAINT signing_checks_post_binding_known
        CHECK (post_binding IN ('one-post', 'any-post')),
    CONSTRAINT signing_checks_revision_counts CHECK (revision >= 0),
    CONSTRAINT signing_checks_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- A protected action waiting for, or done with, its signatures. The Post is
-- the election, the country the area. `rule_snapshot` is the rule the request
-- was created under; `permission_label` is the Post's, so readers see only
-- the Posts they may see.
CREATE TABLE sequent_backend.signing_request (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    action text NOT NULL,
    election_id uuid,
    area_id uuid,
    trustee_id uuid,
    -- What one request stands for: a new one for the same key supersedes the
    -- one waiting.
    scope_key text NOT NULL,
    subject jsonb NOT NULL,
    canonical_payload text NOT NULL,
    payload_sha256 text NOT NULL,
    document_id uuid,
    document_sha256 text,
    code text NOT NULL,
    config_revision text,
    rule_revision bigint NOT NULL,
    rule_snapshot jsonb NOT NULL,
    required integer NOT NULL,
    status text NOT NULL DEFAULT 'waiting',
    cancel_reason text,
    cancelled_by text,
    requested_by text NOT NULL,
    requested_by_username text NOT NULL,
    requested_by_name text,
    -- NULL: waits without a time limit.
    expires_at timestamptz,
    completed_at timestamptz,
    executed_at timestamptz,
    execution_result jsonb,
    task_execution_id uuid,
    open_failures integer NOT NULL DEFAULT 0,
    permission_label text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    -- Referenced by the rows of the request, which stay in its event.
    CONSTRAINT signing_request_in_its_event UNIQUE (tenant_id, election_event_id, id),
    CONSTRAINT signing_request_action_known CHECK (action IN (
        'initialize-voting', 'open-voting', 'close-voting',
        'generate-election-returns', 'generate-reports', 'transmit-results',
        'approve-voter', 'approve-configuration', 'key-ceremony', 'tally-key'
    )),
    CONSTRAINT signing_request_status_known CHECK (status IN (
        'waiting', 'completed', 'executed', 'cancelled', 'expired', 'failed'
    )),
    CONSTRAINT signing_request_cancel_reason_known CHECK (cancel_reason IN (
        'by-requester', 'by-operator', 'rule-changed', 'payload-changed', 'superseded',
        'certificate-revoked'
    )),
    -- A cancelled request says why, and only a cancelled one does.
    CONSTRAINT signing_request_cancelled_with_reason
        CHECK ((status = 'cancelled') = (cancel_reason IS NOT NULL)),
    CONSTRAINT signing_request_needs_a_signature CHECK (required >= 1),
    -- A request that got its signatures says when; an executed one says
    -- when it ran.
    CONSTRAINT signing_request_completed_when CHECK (
        status NOT IN ('completed', 'executed', 'failed') OR completed_at IS NOT NULL
    ),
    CONSTRAINT signing_request_executed_when
        CHECK (status <> 'executed' OR executed_at IS NOT NULL),
    CONSTRAINT signing_request_open_failures_count CHECK (open_failures >= 0),
    CONSTRAINT signing_request_payload_sha256_hex
        CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT signing_request_document_sha256_hex
        CHECK (document_sha256 ~ '^[0-9a-f]{64}$'),
    -- A request of a Post or a country goes with it; the electoral log keeps
    -- its steps.
    CONSTRAINT signing_request_of_its_election
        FOREIGN KEY (tenant_id, election_event_id, election_id)
        REFERENCES sequent_backend.election (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    CONSTRAINT signing_request_of_its_area
        FOREIGN KEY (tenant_id, election_event_id, area_id)
        REFERENCES sequent_backend.area (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    CONSTRAINT signing_request_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- One waiting request per action and scope.
CREATE UNIQUE INDEX signing_request_one_waiting
    ON sequent_backend.signing_request (tenant_id, election_event_id, action, scope_key)
    WHERE status = 'waiting';
CREATE INDEX signing_request_by_event
    ON sequent_backend.signing_request (tenant_id, election_event_id, created_at);
-- What the expiry job looks at.
CREATE INDEX signing_request_waiting_expiry
    ON sequent_backend.signing_request (expires_at)
    WHERE status = 'waiting' AND expires_at IS NOT NULL;

-- A staff certificate registered to one account, for one Post or for the
-- event. A key (SPKI) and a holder (subject) belong to one account across the
-- tenant, unless a Security Officer linked them (`linked_to`: the first
-- account).
CREATE TABLE sequent_backend.staff_certificate (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    user_id text NOT NULL,
    username text NOT NULL,
    user_display_name text,
    election_id uuid,
    fingerprint_sha256 text NOT NULL,
    spki_sha256 text NOT NULL,
    holder_sha256 text NOT NULL,
    serial text NOT NULL,
    subject text NOT NULL,
    issuer text NOT NULL,
    not_before timestamptz NOT NULL,
    not_after timestamptz NOT NULL,
    pem text NOT NULL,
    status text NOT NULL DEFAULT 'active',
    registration text NOT NULL,
    linked_to text,
    registered_by text NOT NULL,
    registered_by_name text,
    registered_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    revoked_by text,
    revoked_by_name text,
    revoked_at timestamptz,
    revoke_reason text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT staff_certificate_in_its_event UNIQUE (tenant_id, election_event_id, id),
    CONSTRAINT staff_certificate_status_known CHECK (status IN ('active', 'revoked')),
    CONSTRAINT staff_certificate_registration_known
        CHECK (registration IN ('first-use', 'security-officer')),
    -- Only a Security Officer links a certificate to the holder's other
    -- account.
    CONSTRAINT staff_certificate_linked_by_security_officer CHECK (
        linked_to IS NULL OR (registration = 'security-officer' AND linked_to <> user_id)
    ),
    -- A revoked registration says when, and only a revoked one does.
    CONSTRAINT staff_certificate_revoked_when
        CHECK ((status = 'revoked') = (revoked_at IS NOT NULL)),
    CONSTRAINT staff_certificate_fingerprint_sha256_hex
        CHECK (fingerprint_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT staff_certificate_spki_sha256_hex
        CHECK (spki_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT staff_certificate_holder_sha256_hex
        CHECK (holder_sha256 ~ '^[0-9a-f]{64}$'),
    -- A registration for a deleted Post stays, for no Post.
    CONSTRAINT staff_certificate_for_its_election
        FOREIGN KEY (tenant_id, election_event_id, election_id)
        REFERENCES sequent_backend.election (tenant_id, election_event_id, id)
        ON DELETE SET NULL (election_id),
    CONSTRAINT staff_certificate_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- One active registration of a certificate per event, besides the links a
-- Security Officer made to the holder's other accounts; and one per account.
CREATE UNIQUE INDEX staff_certificate_one_active
    ON sequent_backend.staff_certificate (tenant_id, election_event_id, fingerprint_sha256)
    WHERE status = 'active' AND linked_to IS NULL;
CREATE UNIQUE INDEX staff_certificate_one_active_per_user
    ON sequent_backend.staff_certificate
        (tenant_id, election_event_id, fingerprint_sha256, user_id)
    WHERE status = 'active';
-- Tenant-wide lookups of who a key or a holder is registered to, and of the
-- revoked certificates and keys, which first use can't register again.
CREATE INDEX staff_certificate_active_by_spki
    ON sequent_backend.staff_certificate (tenant_id, spki_sha256)
    WHERE status = 'active';
CREATE INDEX staff_certificate_active_by_holder
    ON sequent_backend.staff_certificate (tenant_id, holder_sha256)
    WHERE status = 'active';
CREATE INDEX staff_certificate_revoked_by_fingerprint
    ON sequent_backend.staff_certificate (tenant_id, fingerprint_sha256)
    WHERE status = 'revoked';
CREATE INDEX staff_certificate_revoked_by_spki
    ON sequent_backend.staff_certificate (tenant_id, spki_sha256)
    WHERE status = 'revoked';
CREATE INDEX staff_certificate_by_user
    ON sequent_backend.staff_certificate (tenant_id, election_event_id, user_id);

-- One signature of a request. Each person fills one slot: per request, an
-- account, a certificate, a key and a holder sign once.
CREATE TABLE sequent_backend.signing_approval (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    request_id uuid NOT NULL,
    user_id text NOT NULL,
    username text NOT NULL,
    display_name text,
    auth_time timestamptz,
    certificate_id uuid NOT NULL,
    certificate_pem text NOT NULL,
    chain_pem text NOT NULL,
    fingerprint_sha256 text NOT NULL,
    spki_sha256 text NOT NULL,
    holder_sha256 text NOT NULL,
    algorithm text NOT NULL,
    payload_signature bytea NOT NULL,
    document_signature bytea,
    pdf_cms bytea,
    revocation_status text NOT NULL,
    signed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT signing_approval_one_per_user UNIQUE (request_id, user_id),
    CONSTRAINT signing_approval_one_per_certificate UNIQUE (request_id, fingerprint_sha256),
    CONSTRAINT signing_approval_one_per_key UNIQUE (request_id, spki_sha256),
    CONSTRAINT signing_approval_one_per_holder UNIQUE (request_id, holder_sha256),
    CONSTRAINT signing_approval_algorithm_known
        CHECK (algorithm IN ('rsa-pkcs1-sha256', 'ecdsa-p256-sha256')),
    CONSTRAINT signing_approval_revocation_status_known
        CHECK (revocation_status IN ('checked', 'unchecked')),
    CONSTRAINT signing_approval_fingerprint_sha256_hex
        CHECK (fingerprint_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT signing_approval_spki_sha256_hex
        CHECK (spki_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT signing_approval_holder_sha256_hex
        CHECK (holder_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT signing_approval_of_its_request
        FOREIGN KEY (tenant_id, election_event_id, request_id)
        REFERENCES sequent_backend.signing_request (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    CONSTRAINT signing_approval_with_its_certificate
        FOREIGN KEY (tenant_id, election_event_id, certificate_id)
        REFERENCES sequent_backend.staff_certificate (tenant_id, election_event_id, id),
    CONSTRAINT signing_approval_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- The PAdES revisions of a request's PDF: the base document, a revision
-- prepared for one signer, and the signed ones.
CREATE TABLE sequent_backend.signing_document_revision (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    request_id uuid NOT NULL,
    revision integer NOT NULL,
    document_id uuid NOT NULL,
    sha256 text NOT NULL,
    state text NOT NULL,
    prepared_for_user text,
    byte_range jsonb,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT signing_document_revision_once UNIQUE (request_id, revision),
    CONSTRAINT signing_document_revision_counts CHECK (revision >= 0),
    CONSTRAINT signing_document_revision_state_known
        CHECK (state IN ('base', 'prepared', 'signed')),
    CONSTRAINT signing_document_revision_sha256_hex CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT signing_document_revision_of_its_request
        FOREIGN KEY (tenant_id, election_event_id, request_id)
        REFERENCES sequent_backend.signing_request (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    CONSTRAINT signing_document_revision_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- The last CRL fetched from one distribution point of a staff issuer; it goes
-- with its issuer. `issuer_fingerprint` is the issuer's, for display.
CREATE TABLE sequent_backend.staff_crl (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    issuer_id uuid NOT NULL,
    issuer_fingerprint text NOT NULL,
    url text NOT NULL,
    der bytea,
    this_update timestamptz,
    next_update timestamptz,
    fetched_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    status text NOT NULL,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT staff_crl_one_per_url UNIQUE (tenant_id, election_event_id, url),
    CONSTRAINT staff_crl_status_known CHECK (status IN ('ok', 'unavailable')),
    CONSTRAINT staff_crl_of_its_issuer
        FOREIGN KEY (tenant_id, election_event_id, issuer_id)
        REFERENCES sequent_backend.certificate_authority (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    CONSTRAINT staff_crl_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- Electoral log entries of signing steps, written in the step's transaction
-- and posted in `id` order by the outbox worker. Each step writes a USER
-- entry (0) and a SYSTEM entry (1) under one step_id; SYSTEM entries carry no
-- user. `body` holds the entry's description and details.
CREATE TABLE sequent_backend.signing_log_outbox (
    id bigserial NOT NULL,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    step_id uuid NOT NULL,
    entry smallint NOT NULL,
    statement_kind text NOT NULL,
    event_type text NOT NULL,
    log_type text NOT NULL,
    user_id text,
    username text,
    election_id uuid,
    area_id uuid,
    body jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    posted_at timestamptz,
    attempts integer NOT NULL DEFAULT 0,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT signing_log_outbox_one_entry_per_step UNIQUE (step_id, entry),
    CONSTRAINT signing_log_outbox_entry_known CHECK (entry IN (0, 1)),
    -- electoral_log's SigningStatementKind: an unknown kind would stop the
    -- worker at that row.
    CONSTRAINT signing_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'SigningRequestCreated', 'SigningCertificateOpenFailed', 'SigningRequestSigned',
        'SigningSignatureRefused', 'SigningCertificateRegistered', 'SigningHandover',
        'SigningRequestCancelled', 'SigningRequestExpired', 'SigningRequestCompleted',
        'SigningActionExecuted', 'SigningRuleChanged', 'SigningPermissionChanged',
        'SigningIssuerChanged', 'SigningChecksChanged', 'SigningCertificateRevoked',
        'SigningRequestsExported'
    )),
    CONSTRAINT signing_log_outbox_event_type_known CHECK (event_type IN ('USER', 'SYSTEM')),
    CONSTRAINT signing_log_outbox_log_type_known CHECK (log_type IN ('INFO', 'ERROR')),
    -- The USER entry is entry 0 and names its user; the SYSTEM entry names none.
    CONSTRAINT signing_log_outbox_entry_is_its_event_type
        CHECK ((entry = 0) = (event_type = 'USER')),
    CONSTRAINT signing_log_outbox_user_on_user_entries
        CHECK ((event_type = 'USER') = (user_id IS NOT NULL)),
    CONSTRAINT signing_log_outbox_attempts_count CHECK (attempts >= 0),
    CONSTRAINT signing_log_outbox_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);

-- What the worker still has to post.
CREATE INDEX signing_log_outbox_unposted
    ON sequent_backend.signing_log_outbox (tenant_id, election_event_id, id)
    WHERE posted_at IS NULL;
