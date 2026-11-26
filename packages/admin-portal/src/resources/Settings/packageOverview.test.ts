// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IPermissions} from "@/types/keycloak"
import {record, refusedPreview} from "./configurationPackage.fixture"
import {PACKAGE_SECTIONS, readApplication, type SectionView} from "./configurationPackage"
import {
    headerActions,
    lastAppliedReceipt,
    missingPackagePermissions,
    orderedSectionBadges,
    runStages,
    sectionBadge,
} from "./packageOverview"

const section = (state: string, extra: Partial<SectionView> = {}): SectionView => ({
    section: "trustees",
    members: [],
    state,
    earlierApplied: false,
    submissions: [],
    ...extra,
})

describe("sectionBadge reads only the recorded section state", () => {
    it.each<[string, Partial<SectionView>, boolean, string]>([
        ["applied", {}, false, "applied"],
        ["unchanged", {}, false, "unchanged"],
        ["previewed", {preview: {needsApply: true, changes: [], blockers: []}}, false, "willChange"],
        ["previewed", {preview: {needsApply: false, changes: [], blockers: []}}, false, "unchanged"],
        ["blocked", {}, false, "blocked"],
        ["missing", {}, false, "missing"],
        ["not-included", {}, false, "notInPackage"],
        ["failed", {}, false, "failed"],
        ["unknown", {}, false, "notConfirmed"],
        ["submitting", {}, true, "applying"],
        // A run that ended with a submission still open is not confirmed, never applied.
        ["submitting", {}, false, "notConfirmed"],
        ["pending", {}, true, "waiting"],
        ["pending", {}, false, "notStarted"],
        ["something-new", {}, false, "notConfirmed"],
    ])("%s (running %s) → %s", (state, extra, running, expected) => {
        expect(sectionBadge(section(state, extra), running)).toBe(expected)
    })

    it("says nothing was run for a section absent from the record", () => {
        expect(sectionBadge(undefined, false)).toBe("notRun")
    })
})

it("lists every section once, in the platform's apply order, whatever the record's order", () => {
    const view = readApplication(record("applied"))
    const reversed = view && {...view, sections: [...view.sections].reverse()}
    const badges = orderedSectionBadges(reversed, false)
    expect(badges.map((b) => b.section)).toEqual([...PACKAGE_SECTIONS])
    expect(badges.find((b) => b.section === "tenant-configuration")?.badge).toBe("applied")
    expect(badges.find((b) => b.section === "election-event")?.badge).toBe("applied")
    expect(badges.find((b) => b.section === "trustees")?.badge).toBe("notRun")
})

it("folds a refused preview's blocked and missing rows into the ordered sections", () => {
    const view = readApplication(refusedPreview(["unknown role x"], {missingTrustees: true}))
    const badges = orderedSectionBadges(view, false)
    expect(badges.find((b) => b.section === "tenant-configuration")?.badge).toBe("blocked")
    expect(badges.find((b) => b.section === "trustees")?.badge).toBe("missing")
})

describe("runStages: Package → Preview → Review → Apply", () => {
    const states = (phase: Parameters<typeof runStages>[0], mode?: "preview" | "apply") =>
        runStages(phase, mode).map((stage) => `${stage.step}:${stage.state}`)

    it("has no stage done before a package is chosen", () => {
        expect(states("choose")).toEqual([
            "package:current",
            "preview:pending",
            "review:pending",
            "apply:pending",
        ])
    })
    it("marks a blocked preview as blocked, never reviewable", () => {
        expect(states("previewBlocked", "preview")).toEqual([
            "package:done",
            "preview:blocked",
            "review:pending",
            "apply:pending",
        ])
    })
    it("marks an out-of-date preview as stale", () => {
        expect(states("previewAgain", "preview")[1]).toBe("preview:stale")
    })
    it("puts Review as current once a preview may be applied", () => {
        expect(states("review", "preview")).toEqual([
            "package:done",
            "preview:done",
            "review:current",
            "apply:pending",
        ])
    })
    it("reports a genuine failure on the step that failed", () => {
        expect(states("attention", "apply")[3]).toBe("apply:attention")
        expect(states("attention", "preview")[1]).toBe("preview:attention")
    })
    it("claims Apply done only on a completed apply", () => {
        expect(states("complete", "apply")).toEqual([
            "package:done",
            "preview:done",
            "review:done",
            "apply:done",
        ])
    })
})

describe("headerActions", () => {
    it("offers Review and apply when the preview may be applied", () => {
        expect(headerActions("review")).toEqual({primary: "reviewAndApply", secondary: "upload"})
    })
    it("never offers Apply for a blocked preview", () => {
        const actions = headerActions("previewBlocked")
        expect(actions).toEqual({primary: "importFixed", secondary: "changeRequirements"})
        expect(Object.values(actions)).not.toContain("reviewAndApply")
    })
    it.each([
        "choose",
        "complete",
        "attention",
        "previewAgain",
        "trustBlocked",
        "waitingForTrust",
        "starting",
        "previewing",
        "applying",
    ] as const)("%s keeps Import package as the primary action", (phase) => {
        expect(headerActions(phase).primary).toBe("upload")
    })
})

describe("lastAppliedReceipt: what is applied on this tenant", () => {
    const row = (
        id: string,
        annotations: unknown,
        execution_status = "SUCCESS",
        created_at = `2026-11-2${id.length}T10:00:00Z`
    ) => ({id, created_at, execution_status, executed_by_user: "ops.lead", annotations})

    it("is the newest successfully applied run, skipping later previews and failures", () => {
        const rows = [
            row("preview-later", record("previewed")),
            row("failed-apply", record("failed"), "FAILED"),
            row("applied-1", record("applied")),
            row("applied-0", record("applied")),
        ]
        expect(lastAppliedReceipt(rows)).toEqual({
            taskId: "applied-1",
            appliedAt: rows[2].created_at,
            appliedBy: "ops.lead",
            packageSha256: "abcdef0123456789abcdef",
        })
    })
    it("is unknown, not empty, while the history has not answered", () => {
        expect(lastAppliedReceipt(undefined)).toBeUndefined()
    })
    it("is null when the recent history holds no applied run", () => {
        expect(lastAppliedReceipt([row("p", record("previewed"))])).toBeNull()
    })
    it("never treats a bare SUCCESS without an applied record as applied", () => {
        expect(lastAppliedReceipt([row("bare", {}, "SUCCESS")])).toBeNull()
    })
})

it("names exactly the grants the operator lacks on the tenant", () => {
    const held = new Set<IPermissions>([IPermissions.TENANT_WRITE, IPermissions.USER_WRITE])
    const isAuthorized = (_: boolean, tenant: string | null, permission: IPermissions) =>
        tenant === "t1" && held.has(permission)
    expect(missingPackagePermissions("t1", isAuthorized)).toEqual([
        "role-write",
        "trustee-create",
        "trustee-write",
        "election-event-create",
    ])
    expect(missingPackagePermissions(null, isAuthorized)).toEqual([])
})
