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
import {buildComelecTenantConfig, type TenantConfigPreflight} from "@/services/ComelecTenantConfig"

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

export interface ComelecConfigStage {
    name: "preflight" | "tenant-config" | "trustees"
    status: "running" | "success" | "reused"
    detail: string
}

export interface ComelecConfigResult {
    preflight: TenantConfigPreflight
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
        const {data} = await client.query({
            query: GET_TASK_BY_ID,
            variables: {task_id: taskId},
            fetchPolicy: "network-only",
        })
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

export interface ComelecConfigDeps {
    client: ApolloClient<unknown>
    tenantId: string
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
        throw new ComelecConfigError(
            "preflight",
            "the selected tenant has no trustee with both a signing and a share-encryption public key; " +
                "approved public trustee keys are required before importing (none are invented here)"
        )
    }
    return reusable
}

/**
 * Run the preflight and, only if it passes, the tenant-config stages for the selected
 * tenant. Throws `ComelecConfigError` with the failing stage (and any pending task id)
 * so the caller stops the whole flow and shows it; the event import must not proceed
 * when this throws.
 */
export async function runComelecConfigStages(
    deps: ComelecConfigDeps
): Promise<ComelecConfigResult> {
    const {client, tenantId, putFile, onStage} = deps
    const now = deps.now ?? (() => Date.now())
    const sleep = deps.sleep ?? ((ms: number) => new Promise((resolve) => setTimeout(resolve, ms)))

    // ---- Preflight (no mutation) ----
    const {file, preflight} = await buildComelecTenantConfig(tenantId)
    const reusedTrustees = await preflightTrustees(client, tenantId)
    onStage?.({
        name: "preflight",
        status: "success",
        detail:
            `Will apply ${preflight.groups.length} groups / ${preflight.roles.length} realm roles ` +
            `and realm display "${preflight.realmDisplayName}". ` +
            `Reusing ${reusedTrustees.length} existing trustee(s): ${reusedTrustees
                .map((trustee) => trustee.name || trustee.id)
                .join(", ")}.`,
    })

    // ---- Mutate: upload + import_tenant_config (roles + keycloak; tenant preserved) ----
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
    const {data: importData, errors} = await client.mutate({
        mutation: IMPORT_TENANT_CONFIG,
        variables: {
            tenantId,
            documentId,
            importConfigurations: {
                include_roles: true,
                include_keycloak: true,
                include_tenant: false,
            },
        },
    })
    const importResult = importData?.import_tenant_config
    if (errors?.length || importResult?.error) {
        throw new ComelecConfigError(
            "tenant-config-import",
            importResult?.error || "the tenant-config import was refused"
        )
    }
    const tenantConfigTaskId: string | undefined = importResult?.task_execution?.id
    if (!tenantConfigTaskId) {
        // No task id means the submission outcome is unknown — never a success.
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

    return {preflight, reusedTrustees, tenantConfigTaskId}
}
