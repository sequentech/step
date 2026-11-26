// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IPermissions} from "@/types/keycloak"
import {record, refusedPreview} from "./configurationPackage.fixture"
import {
    PACKAGE_SECTIONS,
    readApplication,
    sectionsToChange,
    type SectionView,
} from "./configurationPackage"
import {
    changingSections,
    headerActions,
    isConfirmedApply,
    lastAppliedReceipt,
    readRunIdentity,
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
        [
            "previewed",
            {preview: {needsApply: true, changes: [], blockers: []}},
            false,
            "willChange",
        ],
        [
            "previewed",
            {preview: {needsApply: false, changes: [], blockers: []}},
            false,
            "unchanged",
        ],
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
    const badges = orderedSectionBadges(reversed, false, "SUCCESS")
    expect(badges.map((b) => b.section)).toEqual([...PACKAGE_SECTIONS])
    expect(badges.find((b) => b.section === "tenant-configuration")?.badge).toBe("applied")
    expect(badges.find((b) => b.section === "election-event")?.badge).toBe("applied")
    expect(badges.find((b) => b.section === "trustees")?.badge).toBe("notRun")
})

it("folds a refused preview's blocked and missing rows into the ordered sections", () => {
    const view = readApplication(refusedPreview(["unknown role x"], {missingTrustees: true}))
    const badges = orderedSectionBadges(view, false, "FAILED")
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
    it("offers Preview again for an out-of-date preview, never Apply", () => {
        expect(headerActions("previewAgain")).toEqual({
            primary: "previewAgain",
            secondary: "upload",
        })
    })
    it("routes a missing signer roster to Signing trust, keeping import disabled", () => {
        expect(headerActions("trustBlocked", "missingRoster")).toEqual({
            primary: "openTrust",
            secondary: "upload",
        })
    })
    it("offers Check again when the trust check failed", () => {
        expect(headerActions("trustBlocked", "checkFailed")).toEqual({
            primary: "upload",
            secondary: "checkAgain",
        })
        expect(headerActions("trustBlocked")).toEqual(headerActions("trustBlocked", "checkFailed"))
    })
    it.each([
        "choose",
        "complete",
        "attention",
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

describe("readRunIdentity: only what the platform persisted on the run", () => {
    it("reads the bound requirements and signer roster generation", () => {
        const annotations = record("previewed")
        Object.assign(annotations.application, {
            managedRequirements: {
                sections: ["trustees", "tenant-configuration"],
                census: "required",
            },
            managedRosterBinding: {sha256: "ab".repeat(32), generation: 3, requiredApprovals: 2},
        })
        expect(readRunIdentity(annotations)).toEqual({
            packageSha256: "abcdef0123456789abcdef",
            requirements: {sections: ["tenant-configuration", "trustees"], census: "required"},
            roster: {sha256: "ab".repeat(32), generation: 3, requiredApprovals: 2},
        })
    })
    it("omits what a run did not record instead of guessing", () => {
        expect(readRunIdentity(record("applied"))).toEqual({
            packageSha256: "abcdef0123456789abcdef",
        })
        expect(readRunIdentity({})).toBeNull()
    })
    it("ignores malformed bindings and unknown sections", () => {
        const annotations = record("previewed")
        Object.assign(annotations.application, {
            managedRequirements: {sections: ["tenant-configuration", "bogus"], census: "required"},
            managedRosterBinding: {sha256: "not-a-digest", generation: 3, requiredApprovals: 2},
        })
        expect(readRunIdentity(annotations)).toEqual({packageSha256: "abcdef0123456789abcdef"})
    })
})

it("lists the sections a preview would change, in apply order", () => {
    const view = readApplication(record("previewed"))
    const reversed = view && {...view, sections: [...view.sections].reverse()}
    expect(changingSections(reversed)).toEqual(["tenant-configuration"])
    expect(changingSections(null)).toEqual([])
})

it("lists and counts messaging account changes the confirmation shows, even without needsApply", () => {
    const annotations = record("previewed")
    annotations.application.sections.push({
        section: "messaging-accounts",
        members: ["tenant_messaging_accounts.json"],
        status: {
            state: "previewed",
            preview: {
                needsApply: false,
                blockers: [],
                changes: [{reference: "package:ses", action: "replace", channel: "EMAIL"}],
            },
        },
        history: {submissions: [], inFlight: null, applied: false},
    } as never)
    const view = readApplication(annotations)!
    expect(changingSections(view)).toEqual(["tenant-configuration", "messaging-accounts"])
    expect(sectionsToChange(view)).toBe(changingSections(view).length)
})

it("does not list a messaging section whose preview proposes no account change", () => {
    const annotations = record("previewed")
    annotations.application.sections.push({
        section: "messaging-accounts",
        members: [],
        status: {state: "previewed", preview: {needsApply: false, blockers: [], changes: []}},
        history: {submissions: [], inFlight: null, applied: false},
    } as never)
    const view = readApplication(annotations)!
    expect(changingSections(view)).toEqual(["tenant-configuration"])
    expect(sectionsToChange(view)).toBe(1)
})

describe("applied is claimed only when the record and the task row agree", () => {
    const applied = () => readApplication(record("applied"))
    it.each([
        ["SUCCESS", true],
        ["FAILED", false],
        ["CANCELLED", false],
        ["IN_PROGRESS", false],
        [undefined, false],
    ])("task %s → confirmed %s", (status, confirmed) => {
        expect(isConfirmedApply(applied(), status)).toBe(confirmed)
        const badge = orderedSectionBadges(applied(), false, status).find(
            (row) => row.section === "tenant-configuration"
        )?.badge
        expect(badge).toBe(confirmed ? "applied" : "notConfirmed")
    })
    it("keeps a partially failed run's genuinely applied sections", () => {
        const failed = readApplication(record("failed"))
        const rows = orderedSectionBadges(failed, false, "FAILED")
        expect(rows.find((row) => row.section === "tenant-configuration")?.badge).toBe("applied")
        expect(rows.find((row) => row.section === "election-event")?.badge).toBe("failed")
    })
    it("never counts a preview as an application", () => {
        expect(isConfirmedApply(readApplication(record("previewed")), "SUCCESS")).toBe(false)
    })
})

it("badges a previewed Messaging account change as Will change, as the confirmation lists it", () => {
    expect(
        sectionBadge(
            {
                section: "messaging-accounts",
                members: [],
                state: "previewed",
                preview: {
                    needsApply: false,
                    blockers: [],
                    changes: [{reference: "package:ses", action: "replace"}],
                },
                earlierApplied: false,
                submissions: [],
            },
            false
        )
    ).toBe("willChange")
})
