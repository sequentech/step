/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    COMPLETE_DELIVERY,
    PACKAGE_PERMISSIONS,
    changeLines,
    isFinished,
    lastLogLine,
    mayApply,
    mayApplyPackages,
    readApplication,
    recalledSubmission,
    rememberSubmission,
    safeText,
    sectionsToChange,
    shaMatches,
} from "./configurationPackage"
import {record} from "./configurationPackage.fixture"

describe("reading the platform's application record", () => {
    it("is null until the task has one", () => {
        expect(readApplication(null)).toBeNull()
        expect(readApplication({})).toBeNull()
    })

    it("reads sections, members, task ids, reconciliation and blockers", () => {
        const view = readApplication(record("applied"))!
        expect(view.outcome).toBe("applied")
        expect(view.members.map((m) => m.path)).toEqual([
            "tenant_platform-t1.json",
            "manifest.json",
            "export_voters-e1.csv",
        ])
        expect(view.members[0]).toMatchObject({
            disposition: "applied",
            section: "tenant-configuration",
        })
        expect(view.sections.map((s) => [s.section, s.state, s.taskId, s.reconciled])).toEqual([
            ["tenant-configuration", "applied", "task-tc", true],
            ["election-event", "applied", "task-ev", false],
        ])
        const blocked = readApplication(record("previewed", ["unknown role x"]))!
        expect(blocked.blockers).toEqual(["tenant-configuration: unknown role x"])
        expect(sectionsToChange(readApplication(record("previewed"))!)).toBe(1)
    })

    it("reads a failed or unsettled variant's task_id, as serde writes it", () => {
        expect(readApplication(record("failed"))!.sections[1]).toMatchObject({
            state: "failed",
            error: "import failed",
            taskId: "task-ev",
        })
        expect(readApplication(record("unknown"))!.sections[0]).toMatchObject({
            state: "unknown",
            taskId: "task-run",
            inFlight: "task-run",
        })
    })

    it("keeps what earlier runs applied", () => {
        const refused = readApplication(record("refused"))!
        expect(refused.sections[0]).toMatchObject({
            state: "unchanged",
            earlierApplied: true,
            submissions: ["task-earlier"],
        })
        expect(refused.sections[1].earlierApplied).toBe(false)
    })
})

describe("when an application may be submitted", () => {
    const view = readApplication(record("previewed"))!
    const previewed = {tenantId: "t1", documentId: "doc-1", requirements: COMPLETE_DELIVERY, view}

    it("only after a preview without blockers, of the same tenant, document and requirements", () => {
        expect(mayApply(null, "t1", "doc-1", COMPLETE_DELIVERY)).toBe(false)
        expect(mayApply(previewed, "t1", "doc-1", COMPLETE_DELIVERY)).toBe(true)
        expect(mayApply(previewed, "t2", "doc-1", COMPLETE_DELIVERY)).toBe(false)
        expect(mayApply(previewed, "t1", "doc-2", COMPLETE_DELIVERY)).toBe(false)
        expect(mayApply(previewed, "t1", "doc-1", {...COMPLETE_DELIVERY, census: "optional"})).toBe(
            false
        )
        expect(
            mayApply(previewed, "t1", "doc-1", {
                ...COMPLETE_DELIVERY,
                sections: COMPLETE_DELIVERY.sections.slice(1),
            })
        ).toBe(false)
        const blocked = {...previewed, view: readApplication(record("previewed", ["x"]))!}
        expect(mayApply(blocked, "t1", "doc-1", COMPLETE_DELIVERY)).toBe(false)
        const refused = {...previewed, view: readApplication(record("refused"))!}
        expect(mayApply(refused, "t1", "doc-1", COMPLETE_DELIVERY)).toBe(false)
    })

    it("only with every permission the route requires, on the tenant", () => {
        expect(mayApplyPackages("t1", () => true)).toBe(true)
        for (const missing of PACKAGE_PERMISSIONS) {
            expect(mayApplyPackages("t1", (_, __, role) => role !== missing)).toBe(false)
        }
        expect(mayApplyPackages(null, () => true)).toBe(false)
    })
})

describe("the submission a reload resumes", () => {
    beforeEach(() => sessionStorage.clear())

    it("is kept per tenant", () => {
        const active = {
            taskId: "task-1",
            mode: "apply" as const,
            documentId: "doc-1",
            requirements: COMPLETE_DELIVERY,
        }
        rememberSubmission("t1", active)
        expect(recalledSubmission("t1")).toEqual(active)
        expect(recalledSubmission("t2")).toBeNull()
        expect(recalledSubmission(null)).toBeNull()
        sessionStorage.setItem("configurationPackage.active.t3", "{not json")
        expect(recalledSubmission("t3")).toBeNull()
    })
})

// Records windmill's own code serialized (configuration_package_apply_tests.rs
// the_admin_portal_fixture_is_what_the_task_records), not written by hand.
// eslint-disable-next-line @typescript-eslint/no-var-requires
const golden: Record<string, unknown> = require("./configurationPackage.golden.json")

describe("records as the platform serializes them", () => {
    const read = (name: string) => readApplication({application: golden[name]})!
    const states = (name: string) => read(name).sections.map((s) => [s.section, s.state, s.taskId])

    it("reads every outcome", () => {
        for (const name of ["applied", "previewed", "refused", "failed", "unknown", "submitting"]) {
            expect(read(name).outcome).toBe(name === "submitting" ? "running" : name)
            expect(read(name).sections).toHaveLength(6)
        }
    })

    it("takes a submission's taskId", () => {
        expect(states("applied")).toEqual(
            [
                "tenant-configuration",
                "tenant-settings",
                "role-permissions",
                "admin-accounts",
                "trustees",
                "election-event",
            ].map((section) => [section, "applied", expect.stringMatching(/^task-/)])
        )
        expect(read("applied").sections[0].submissions).toEqual(["task-TenantConfiguration"])
    })

    it("takes the task_id inside a failed, unknown or submitting status", () => {
        expect(states("failed")[3]).toEqual(["admin-accounts", "failed", "task-AdminAccounts"])
        expect(states("unknown")[5]).toEqual(["election-event", "unknown", "task-ElectionEvent"])
        expect(read("submitting").sections.find((s) => s.state === "submitting")?.taskId).toBe(
            "task-TenantConfiguration"
        )
    })

    it("reads a refusal's blocker and keeps a previewed run applicable", () => {
        expect(read("refused").blockers).toEqual(["role-permissions: unknown role"])
        expect(read("previewed").blockers).toEqual([])
        expect(read("previewed").sections.every((s) => s.preview?.needsApply)).toBe(true)
    })

    it("reads nested members with their container and disposition", () => {
        const member = read("applied").members.find((m) => m.path === "admin_users.csv")
        expect(member).toEqual({
            container: "admin_portal_settings.zip",
            path: "admin_users.csv",
            disposition: "applied",
            section: "admin-accounts",
        })
    })
})

describe("finished tasks", () => {
    it("include a CANCELLED one", () => {
        expect(isFinished("CANCELLED")).toBe(true)
        expect(isFinished("FAILED")).toBe(true)
        expect(isFinished("SUCCESS")).toBe(true)
        expect(isFinished("IN_PROGRESS")).toBe(false)
        expect(isFinished(undefined)).toBe(false)
    })

    it("read a record-less failure's reason from the last log line", () => {
        expect(
            lastLogLine([
                {created_date: "a", log_text: "Task started"},
                {created_date: "b", log_text: "Error: document not found"},
            ])
        ).toBe("Error: document not found")
        expect(lastLogLine(null)).toBeUndefined()
    })
})

describe("the entered SHA-256", () => {
    const view = readApplication({application: golden.previewed})!
    const previewedPackage = {
        tenantId: "t1",
        documentId: "doc",
        requirements: COMPLETE_DELIVERY,
        view,
    }

    it("must be the package the platform read, compared in any case", () => {
        expect(shaMatches(view, null)).toBe(true)
        expect(shaMatches(view, "AE".repeat(32))).toBe(true)
        expect(shaMatches(view, "ff".repeat(32))).toBe(false)
        expect(mayApply(previewedPackage, "t1", "doc", COMPLETE_DELIVERY, "ae".repeat(32))).toBe(
            true
        )
        expect(mayApply(previewedPackage, "t1", "doc", COMPLETE_DELIVERY, "ff".repeat(32))).toBe(
            false
        )
    })
})

describe("proposed changes", () => {
    it("name what each section's preview would change, in the platform's own shapes", () => {
        // configuration_package_sections.rs: tenant settings, roles, admins,
        // the tenant record, trustees and the event.
        expect(
            changeLines([
                {field: "displayName", action: "update", from: "Old", to: "New"},
                {field: "localizationTexts", action: "update"},
            ])
        ).toEqual(["displayName · update · Old → New", "localizationTexts · update"])
        expect(
            changeLines([
                {group: "admin", action: "update", add: ["tenant-write"], remove: ["tally"]},
                {group: "auditor", action: "create", add: ["election-event-read"], remove: []},
            ])
        ).toEqual([
            "admin · update · add: tenant-write · remove: tally",
            "auditor · create · add: election-event-read",
        ])
        expect(changeLines([{username: "operator.one", group: "admin", action: "create"}])).toEqual(
            ["operator.one / admin · create"]
        )
        expect(changeLines([{field: "settings", action: "update", keys: ["i18n"]}])).toEqual([
            "settings · update · keys: i18n",
        ])
        expect(changeLines([{name: "trustee-a", action: "register"}])).toEqual([
            "trustee-a · register",
        ])
        expect(changeLines([{electionEventId: "e1", action: "import", censusSize: 80}])).toEqual([
            "e1 · import · census: 80",
        ])
        expect(changeLines({files: ["tenant_platform-t1.json"]})).toEqual([
            "files: tenant_platform-t1.json",
        ])
    })

    it("never show a value that is not a name", () => {
        const lines = changeLines([
            {
                field: "settings",
                action: "update",
                keys: ["gapi_key"],
                value: {private_key: "-----BEGIN PRIVATE KEY-----"},
                password: "hunter2",
            },
        ])
        expect(lines).toEqual(["settings · update · keys: gapi_key"])
        expect(lines.join()).not.toContain("hunter2")
        expect(lines.join()).not.toContain("PRIVATE")
    })
})

describe("platform text", () => {
    it("redacts token-like runs and bounds its length", () => {
        expect(safeText(`bad secret ${"Ab9".repeat(20)} here`)).toBe("bad secret [redacted] here")
        expect(safeText("x ".repeat(400)).length).toBeLessThanOrEqual(301)
        expect(safeText("import failed")).toBe("import failed")
    })
})
