// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Orchestrate the tenant-level configuration stages of the COMELEC two-file import
// (meta#14593), composing only existing supported operations.
//
// Order matters: everything that can be checked is PREFLIGHTED before any mutation.
//   Preflight (no mutation):
//     - generate + preview the tenant-config archive from the versioned public profile;
//     - read the tenant's existing public trustees (id + both public keys) and decide
//       reuse. If none carry the required public keys, STOP here — approved public
//       trustee keys are never invented and the flow must not proceed as if set up.
//   Mutate (only after preflight passes):
//     - upload + import_tenant_config (roles + keycloak display/localization; the
//       tenant's id/slug/other settings are preserved). A valid task id and a terminal
//       SUCCESS are required; a missing response, missing id, FAILED or timeout is an
//       error or an uncertain outcome — never treated as success. The submitted task id
//       is returned so the caller can reconcile an uncertain submission before retry.
//
// Realm-role existence: import_tenant_config resolves each requested role against the
// realm and surfaces an unknown role as a task failure (see read_roles_config_file),
// so a missing role fails loudly rather than being silently dropped. A pre-mutation
// realm-role listing is not available to this flow; `preflight.roles` records exactly
// which realm roles the archive will request, for review.

import type {ApolloClient} from "@apollo/client"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {IMPORT_TENANT_CONFIG} from "@/queries/ImportTenantConfig"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {GET_TENANT_TRUSTEES} from "@/queries/GetTenantTrustees"
import {GET_TENANT_STATE} from "@/queries/GetTenantState"
import {getPermissions} from "@/queries/GetPermissions"
import {getRoles} from "@/queries/GetRoles"
import {
    buildComelecTenantConfig,
    comelecGroups,
    type TenantConfigPreflight,
    type TenantState,
} from "@/services/ComelecTenantConfig"

export type ComelecConfigStageName =
    | "preflight"
    | "tenant-config-upload"
    | "tenant-config-import"

export class ComelecConfigError extends Error {
    constructor(
        public readonly stage: ComelecConfigStageName,
        message: string,
        /** A submitted task whose outcome is unknown, to reconcile before any retry. */
        public readonly pendingTaskId?: string
    ) {
        super(message)
        this.name = "ComelecConfigError"
    }
}

export interface ReusableTrustee {
    id: string
    name: string
}

export interface GroupRoleChange {
    group: string
    /** Whether the group already exists in the target realm. */
    exists: boolean
    /** Realm roles this import would add to the group. */
    additions: string[]
    /** Existing realm-role mappings this import would REMOVE from the group. */
    removals: string[]
}

export interface RolePreflight {
    /** Profile roles absent from the realm; the import would fail, so the flow stops. */
    missingRoles: string[]
    /** Per profile group, the exact additions and removals against the current realm. */
    changes: GroupRoleChange[]
}

export interface ComelecConfigStage {
    name: "preflight" | "tenant-config" | "trustees"
    status: "running" | "success" | "reused"
    detail: string
}

export interface ComelecConfigResult {
    preflight: TenantConfigPreflight
    /** The realm-role diff computed before mutating (additions/removals per group). */
    rolePreflight: RolePreflight
    /** Existing approved trustees reused, by id; the flow never creates or invents keys. */
    reusedTrustees: ReusableTrustee[]
    /** The import task that completed SUCCESS, recorded for provenance. */
    tenantConfigTaskId: string
}

interface RawTrustee {
    id?: string | null
    name?: string | null
    public_key?: string | null
    share_encryption_public_key?: string | null
}

const TASK_TIMEOUT_MS = 300_000
const TASK_POLL_MS = 3_000

async function awaitTaskSuccess(
    client: ApolloClient<unknown>,
    stage: ComelecConfigStageName,
    taskId: string,
    now: () => number,
    sleep: (ms: number) => Promise<void>
): Promise<void> {
    const deadline = now() + TASK_TIMEOUT_MS
    for (;;) {
        let data: {sequent_backend_tasks_execution?: Array<{execution_status?: string}>}
        try {
            ;({data} = await client.query({
                query: GET_TASK_BY_ID,
                variables: {task_id: taskId},
                fetchPolicy: "network-only",
            }))
        } catch (error) {
            // A transient poll failure must not lose the submitted task: surface it
            // WITH the id so the UI reconciles/resumes rather than resubmitting blind.
            throw new ComelecConfigError(
                stage,
                `lost contact while awaiting the import task; its outcome is unconfirmed (${
                    error instanceof Error ? error.message : "poll failed"
                })`,
                taskId
            )
        }
        const status: string | undefined =
            data?.sequent_backend_tasks_execution?.[0]?.execution_status
        if (status === "SUCCESS") return
        if (status === "FAILED") {
            throw new ComelecConfigError(stage, "the tenant-config import task failed", taskId)
        }
        if (now() >= deadline) {
            // Submitted but unconfirmed: hand back the id so it is reconciled, not retried blind.
            throw new ComelecConfigError(
                stage,
                "timed out awaiting the tenant-config import task; its outcome is unconfirmed",
                taskId
            )
        }
        await sleep(TASK_POLL_MS)
    }
}

/** The preview a confirmed apply acts on; produced by `prepareComelecConfig`. */
export interface ComelecConfigPlan {
    /** The generated tenant-config archive, ready to upload. */
    file: File
    preflight: TenantConfigPreflight
    rolePreflight: RolePreflight
    /** Existing approved trustees that would be reused, by id (never created here). */
    reusedTrustees: ReusableTrustee[]
}

export interface ApplyComelecConfigDeps {
    client: ApolloClient<unknown>
    tenantId: string
    /** A plan from `prepareComelecConfig`, after the operator confirmed it. */
    plan: ComelecConfigPlan
    /** PUT the generated archive to the presigned URL (reuses the screen's uploader). */
    putFile: (url: string, file: File) => Promise<void>
    onStage?: (stage: ComelecConfigStage) => void
    now?: () => number
    sleep?: (ms: number) => Promise<void>
}

/**
 * Read the tenant's existing public trustees and return those usable for reuse — the
 * ones carrying BOTH a signing public key and a share-encryption public key. Throws if
 * none, so the flow stops before mutating rather than importing a config against a
 * tenant with no valid crypto trustees.
 */
async function preflightTrustees(
    client: ApolloClient<unknown>,
    tenantId: string
): Promise<ReusableTrustee[]> {
    const {data} = await client.query({
        query: GET_TENANT_TRUSTEES,
        variables: {tenantId},
        fetchPolicy: "network-only",
    })
    const trustees: RawTrustee[] = data?.sequent_backend_trustee ?? []
    const reusable = trustees
        .filter(
            (trustee) =>
                !!trustee.id &&
                !!trustee.public_key &&
                !!trustee.share_encryption_public_key
        )
        .map((trustee) => ({id: trustee.id as string, name: trustee.name ?? ""}))
    if (reusable.length === 0) {
        // Reuse-only is not full setup: when the tenant has no valid public trustee,
        // name the exact missing public inputs. The flow never invents them, and the
        // supplied SQL+XLSX contain no trustee identities/keys, so these must be
        // supplied as approved public configuration (then registered via the existing
        // public trustee CRUD — not done implicitly here).
        throw new ComelecConfigError(
            "preflight",
            "the selected tenant has no trustee with both public keys. Approved public trustee " +
                "configuration is required before import and is never invented: for each trustee, a " +
                "name/identity, a signing public key and a share-encryption public key. Register them " +
                "via the trustee settings (public CRUD), then retry; the source files provide none."
        )
    }
    return reusable
}

/**
 * Preflight the realm roles BEFORE mutating: confirm every role the profile assigns
 * exists in the target realm (a missing one would fail the import mid-apply), and
 * compute, per profile group, the exact additions and the existing mappings that the
 * import would REMOVE — so the operator previews real changes, not just desired counts.
 */
async function preflightRealmRoles(
    client: ApolloClient<unknown>,
    tenantId: string
): Promise<RolePreflight> {
    const permsResult = await client.query({
        query: getPermissions({tenant_id: tenantId}),
        fetchPolicy: "network-only",
    })
    const available = new Set<string>(
        (permsResult.data?.get_permissions?.items ?? [])
            .map((item: {name?: string | null}) => item.name ?? "")
            .filter((name: string) => name.length > 0)
    )
    const rolesResult = await client.query({
        query: getRoles({tenant_id: tenantId}),
        fetchPolicy: "network-only",
    })
    const currentByGroup = new Map<string, Set<string>>()
    for (const group of rolesResult.data?.get_roles?.items ?? []) {
        if (group?.name) {
            currentByGroup.set(group.name, new Set<string>(group.permissions ?? []))
        }
    }
    const desiredRoles = new Set<string>()
    const changes: GroupRoleChange[] = comelecGroups.map((group) => {
        const desired = new Set(group.realmRoles)
        group.realmRoles.forEach((role) => desiredRoles.add(role))
        const current = currentByGroup.get(group.name)
        return {
            group: group.name,
            exists: current !== undefined,
            additions: group.realmRoles.filter((role) => !current?.has(role)).sort(),
            removals: current
                ? Array.from(current)
                      .filter((role) => !desired.has(role))
                      .sort()
                : [],
        }
    })
    const missingRoles = Array.from(desiredRoles)
        .filter((role) => !available.has(role))
        .sort()
    return {missingRoles, changes}
}

/**
 * PREFLIGHT ONLY — makes no change. Generate the tenant-config archive, compute the
 * exact realm-role additions/removals against the target realm, and confirm the tenant
 * has approved public trustees. Throws `ComelecConfigError("preflight", …)`, naming the
 * problem, when a role is missing or no valid trustee exists. The returned plan is what
 * the UI shows for confirmation before `applyComelecConfig` mutates anything.
 */
async function fetchTenantState(
    client: ApolloClient<unknown>,
    tenantId: string
): Promise<TenantState> {
    const {data} = await client.query({
        query: GET_TENANT_STATE,
        variables: {tenantId},
        fetchPolicy: "network-only",
    })
    const row = data?.sequent_backend_tenant?.[0]
    if (!row?.id) {
        throw new ComelecConfigError("preflight", "the selected tenant was not found")
    }
    return {
        id: row.id,
        slug: row.slug,
        created_at: row.created_at,
        updated_at: row.updated_at,
        labels: row.labels ?? null,
        annotations: row.annotations ?? null,
        is_active: !!row.is_active,
        voting_channels: row.voting_channels ?? null,
        settings: (row.settings as Record<string, unknown> | null) ?? null,
        test: row.test ?? null,
    }
}

export async function prepareComelecConfig(
    client: ApolloClient<unknown>,
    tenantId: string
): Promise<ComelecConfigPlan> {
    // Read the target tenant so the generated tenant_configuration preserves its id/slug
    // and unrelated state while applying only the approved public display_name.
    const targetTenant = await fetchTenantState(client, tenantId)
    const {file, preflight} = await buildComelecTenantConfig(tenantId, targetTenant)
    const rolePreflight = await preflightRealmRoles(client, tenantId)
    if (rolePreflight.missingRoles.length > 0) {
        throw new ComelecConfigError(
            "preflight",
            `the target realm is missing ${rolePreflight.missingRoles.length} role(s) the profile ` +
                `assigns, so the import would fail: ${rolePreflight.missingRoles.join(", ")}`
        )
    }
    const reusedTrustees = await preflightTrustees(client, tenantId)
    return {file, preflight, rolePreflight, reusedTrustees}
}

/**
 * APPLY — only after the operator confirms a plan from `prepareComelecConfig`. Upload
 * the archive and run import_tenant_config (roles + keycloak display/localization; the
 * tenant id/slug/settings are preserved). Requires a valid task id and a terminal
 * SUCCESS; a transport error, missing id, FAILED, timeout or lost poll is an error
 * carrying any known task id for the caller to reconcile — never a silent success and
 * never a blind retry.
 */
export async function applyComelecConfig(
    deps: ApplyComelecConfigDeps
): Promise<ComelecConfigResult> {
    const {client, tenantId, plan, putFile, onStage} = deps
    const now = deps.now ?? (() => Date.now())
    const sleep = deps.sleep ?? ((ms: number) => new Promise((resolve) => setTimeout(resolve, ms)))
    const {file, preflight, rolePreflight, reusedTrustees} = plan

    onStage?.({name: "tenant-config", status: "running", detail: "Uploading tenant configuration…"})
    let documentId: string
    try {
        const {data} = await client.mutate({
            mutation: GET_UPLOAD_URL,
            variables: {
                name: file.name,
                media_type: "application/zip",
                size: file.size,
                is_public: false,
            },
        })
        const url: string | undefined = data?.get_upload_url?.url
        const id: string | undefined = data?.get_upload_url?.document_id
        if (!url || !id) throw new Error("the upload URL response was incomplete")
        await putFile(url, file)
        documentId = id
    } catch (error) {
        throw new ComelecConfigError(
            "tenant-config-upload",
            error instanceof Error ? error.message : "tenant-config upload failed"
        )
    }

    onStage?.({name: "tenant-config", status: "running", detail: "Importing tenant configuration…"})
    let importResult:
        | {error?: string | null; task_execution?: {id?: string} | null}
        | undefined
    try {
        const {data, errors} = await client.mutate({
            mutation: IMPORT_TENANT_CONFIG,
            variables: {
                tenantId,
                documentId,
                importConfigurations: {
                    include_roles: true,
                    include_keycloak: true,
                    // The archive carries a tenant_configuration that preserves the
                    // target id/slug and applies only the approved display_name.
                    include_tenant: true,
                },
            },
        })
        if (errors?.length) {
            throw new Error(errors.map((graphqlError) => graphqlError.message).join(", "))
        }
        importResult = data?.import_tenant_config
    } catch (error) {
        // Submitted but the transport failed before a task id came back: the outcome is
        // explicitly unknown and must be reconciled, not blindly retried.
        throw new ComelecConfigError(
            "tenant-config-import",
            `the tenant-config import submission outcome is unknown (${
                error instanceof Error ? error.message : "transport error"
            }); reconcile before retrying`
        )
    }
    if (importResult?.error) {
        throw new ComelecConfigError("tenant-config-import", importResult.error)
    }
    const tenantConfigTaskId: string | undefined = importResult?.task_execution?.id
    if (!tenantConfigTaskId) {
        throw new ComelecConfigError(
            "tenant-config-import",
            "the tenant-config import returned no task to confirm; its outcome is unknown"
        )
    }
    await awaitTaskSuccess(client, "tenant-config-import", tenantConfigTaskId, now, sleep)
    onStage?.({name: "tenant-config", status: "success", detail: "Tenant configuration imported."})
    onStage?.({
        name: "trustees",
        status: "reused",
        detail: `Reused ${reusedTrustees.length} existing trustee(s); no keys were created.`,
    })
    return {preflight, rolePreflight, reusedTrustees, tenantConfigTaskId}
}
