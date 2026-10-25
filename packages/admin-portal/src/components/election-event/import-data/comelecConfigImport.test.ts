// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApolloClient} from "@apollo/client"
import {
    ComelecConfigError,
    prepareComelecConfig,
    applyComelecConfig,
    type ComelecConfigPlan,
} from "./comelecConfigImport"
import {comelecGroups} from "@/services/ComelecTenantConfig"

// A minimal fake Apollo client. query()/mutate() are routed by the operation name so a
// test can stage the realm state and the task outcome.
interface FakeState {
    permissions: Array<{name: string}>
    roles: Array<{name: string; permissions: string[]}>
    trustees: Array<{
        id: string
        name: string
        public_key: string | null
        share_encryption_public_key: string | null
    }>
    importTask?: {id?: string; error?: string}
    taskStatus: string
    taskQueryThrows?: boolean
}

const opName = (doc: any): string => doc?.definitions?.[0]?.name?.value ?? ""

function fakeClient(state: FakeState): {client: ApolloClient<unknown>; mutations: string[]} {
    const mutations: string[] = []
    const client = {
        async query({query}: {query: any}) {
            const name = opName(query)
            if (name === "getPermissions")
                return {data: {get_permissions: {items: state.permissions}}}
            if (name === "getRoles") return {data: {get_roles: {items: state.roles}}}
            if (name === "TenantTrustees")
                return {data: {sequent_backend_trustee: state.trustees}}
            if (name === "TenantState")
                return {
                    data: {
                        sequent_backend_tenant: [
                            {
                                id: "tenant-1",
                                slug: "comelec",
                                created_at: "2027-01-01T00:00:00+00:00",
                                updated_at: "2027-01-01T00:00:00+00:00",
                                labels: null,
                                annotations: null,
                                is_active: true,
                                voting_channels: null,
                                settings: {existing: "kept"},
                                test: null,
                            },
                        ],
                    },
                }
            if (name === "GetTaskById") {
                if (state.taskQueryThrows) throw new Error("network blip")
                return {
                    data: {
                        sequent_backend_tasks_execution: [{execution_status: state.taskStatus}],
                    },
                }
            }
            return {data: {}}
        },
        async mutate({mutation}: {mutation: any}) {
            const name = opName(mutation)
            mutations.push(name)
            if (name === "GetUploadUrl")
                return {data: {get_upload_url: {url: "https://s3/put", document_id: "doc-1"}}}
            if (name === "ImportTenantConfig")
                return {
                    data: {
                        import_tenant_config: {
                            error: state.importTask?.error ?? null,
                            task_execution: state.importTask?.id ? {id: state.importTask.id} : null,
                        },
                    },
                }
            return {data: {}}
        },
    } as unknown as ApolloClient<unknown>
    return {client, mutations}
}

const allProfileRoles = Array.from(
    new Set(comelecGroups.flatMap((group) => group.realmRoles))
).map((name) => ({name}))

const validTrustee = {
    id: "t1",
    name: "Trustee One",
    public_key: "PUB",
    share_encryption_public_key: "SHARE",
}

const applyDeps = (client: ApolloClient<unknown>, plan: ComelecConfigPlan) => ({
    client,
    tenantId: "tenant-1",
    plan,
    putFile: async () => undefined,
    now: () => 0,
    sleep: async () => undefined,
})

describe("prepareComelecConfig (preflight only — no mutation)", () => {
    it("stops when the tenant has no trustee with both public keys, and names what is missing", async () => {
        const {client, mutations} = fakeClient({
            permissions: allProfileRoles,
            roles: [],
            trustees: [
                {id: "t1", name: "x", public_key: "PUB", share_encryption_public_key: null},
            ],
            taskStatus: "SUCCESS",
        })
        const error = await prepareComelecConfig(client, "tenant-1").catch((e) => e)
        expect(error).toBeInstanceOf(ComelecConfigError)
        expect(error.stage).toBe("preflight")
        expect(error.message).toMatch(/share-encryption public key/)
        expect(mutations).toEqual([]) // preflight mutates nothing
    })

    it("stops when a profile role is absent from the realm", async () => {
        const {client, mutations} = fakeClient({
            permissions: [{name: "only-this-one-role"}],
            roles: [],
            trustees: [validTrustee],
            taskStatus: "SUCCESS",
        })
        const error = await prepareComelecConfig(client, "tenant-1").catch((e) => e)
        expect(error.stage).toBe("preflight")
        expect(mutations).toEqual([])
    })

    it("returns a plan previewing exact removals and the reusable trustee", async () => {
        const {client, mutations} = fakeClient({
            permissions: allProfileRoles,
            roles: [{name: "admin", permissions: ["an-extra-legacy-role"]}],
            trustees: [validTrustee],
            taskStatus: "SUCCESS",
        })
        const plan = await prepareComelecConfig(client, "tenant-1")
        expect(mutations).toEqual([]) // still no mutation before apply
        expect(plan.reusedTrustees).toEqual([{id: "t1", name: "Trustee One"}])
        const adminChange = plan.rolePreflight.changes.find((c) => c.group === "admin")
        expect(adminChange?.exists).toBe(true)
        expect(adminChange?.removals).toContain("an-extra-legacy-role")
    })
})

describe("applyComelecConfig (after a confirmed plan)", () => {
    const cleanRealm = (): FakeState => ({
        permissions: allProfileRoles,
        roles: [],
        trustees: [validTrustee],
        importTask: {id: "task-ok"},
        taskStatus: "SUCCESS",
    })

    it("completes on a valid plan and records the task id", async () => {
        const {client, mutations} = fakeClient(cleanRealm())
        const plan = await prepareComelecConfig(client, "tenant-1")
        const result = await applyComelecConfig(applyDeps(client, plan))
        expect(result.tenantConfigTaskId).toBe("task-ok")
        expect(mutations).toEqual(["GetUploadUrl", "ImportTenantConfig"])
    })

    it("treats a missing import task id as an unknown outcome, never success", async () => {
        const {client} = fakeClient({...cleanRealm(), importTask: {}})
        const plan = await prepareComelecConfig(client, "tenant-1")
        await expect(applyComelecConfig(applyDeps(client, plan))).rejects.toMatchObject({
            stage: "tenant-config-import",
        })
    })

    it("fails and reports the pending task id when the task FAILS", async () => {
        const {client} = fakeClient({
            ...cleanRealm(),
            importTask: {id: "task-9"},
            taskStatus: "FAILED",
        })
        const plan = await prepareComelecConfig(client, "tenant-1")
        const error = await applyComelecConfig(applyDeps(client, plan)).catch((e) => e)
        expect(error.stage).toBe("tenant-config-import")
        expect(error.pendingTaskId).toBe("task-9")
    })

    it("on a transient poll failure keeps the task id for reconcile, never retries blind", async () => {
        const {client} = fakeClient({
            ...cleanRealm(),
            importTask: {id: "task-7"},
            taskQueryThrows: true,
        })
        const plan = await prepareComelecConfig(client, "tenant-1")
        const error = await applyComelecConfig(applyDeps(client, plan)).catch((e) => e)
        expect(error.pendingTaskId).toBe("task-7")
        expect(error.message).toMatch(/unconfirmed/)
    })
})
