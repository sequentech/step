/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    COMPLETE_DELIVERY,
    PACKAGE_PERMISSIONS,
    mayApply,
    mayApplyPackages,
    readApplication,
    recalledSubmission,
    rememberSubmission,
    sectionsToChange,
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
