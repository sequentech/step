// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Application records exactly as windmill's apply_configuration_package task
// serializes them into the task row's annotations.application. The shape
// follows the serde attributes, not a guess:
// - ApplicationRecord, SectionRecord, SectionHistory, SectionPreview,
//   SectionSubmission: rename_all = "camelCase"
//   (windmill/src/services/import/configuration_package_apply.rs).
// - Outcome and ApplyMode: rename_all = "kebab-case".
// - SectionStatus: rename_all = "kebab-case", tag = "state". That renames the
//   variants only (there is no rename_all_fields), so the fields inside a
//   variant keep their Rust names: task_id in Submitting/Failed/Unknown,
//   while a SectionSubmission's is taskId.
// - Section: rename_all = "kebab-case"; Coverage and MemberDisposition:
//   camelCase; Disposition: kebab-case, tag = "kind", content = "section";
//   SectionState: kebab-case, tag = "state"
//   (sequent-core/src/election_config/package_contract.rs).

const coverage = {
    members: [
        {
            container: null,
            path: "tenant_platform-t1.json",
            sha256: "11".repeat(32),
            size: 120,
            disposition: {kind: "applied", section: "tenant-configuration"},
        },
        {
            container: null,
            path: "manifest.json",
            sha256: "22".repeat(32),
            size: 80,
            disposition: {kind: "verification"},
        },
        {
            container: "event.zip",
            path: "export_voters-e1.csv",
            sha256: "33".repeat(32),
            size: 40,
            disposition: {kind: "applied", section: "election-event"},
        },
    ],
    sections: {
        "tenant-configuration": {state: "present", members: ["tenant_platform-t1.json"]},
        "election-event": {state: "present", members: ["event.zip"]},
    },
    census: {state: "present", members: ["export_voters-e1.csv"]},
}

const history = (applied: boolean, taskId?: string) => ({
    submissions: taskId ? [{taskId, detail: {}}] : [],
    inFlight: null,
    applied,
})

const preview = (needsApply: boolean, blockers: string[] = []) => ({
    needsApply,
    changes: [],
    blockers,
})

const application = (outcome: string, mode: string, sections: unknown[]) => ({
    application: {
        format: "sequent.package-application/1",
        tenantId: "t1",
        packageSha256: "abcdef0123456789abcdef",
        mode,
        coverage,
        sections,
        outcome,
    },
})

/** A record of `outcome`; `blockers` go on the tenant configuration's preview. */
export const record = (outcome: string, blockers: string[] = []) => {
    switch (outcome) {
        case "applied":
            return application("applied", "apply", [
                {
                    section: "tenant-configuration",
                    members: ["tenant_platform-t1.json"],
                    status: {
                        state: "applied",
                        submission: {taskId: "task-tc", detail: {}},
                        reconciled: true,
                    },
                    history: history(true, "task-tc"),
                },
                {
                    section: "election-event",
                    members: ["event.zip"],
                    status: {
                        state: "applied",
                        submission: {taskId: "task-ev", detail: {}},
                        reconciled: false,
                    },
                    history: history(true, "task-ev"),
                },
            ])
        case "failed":
            // Partial: the tenant configuration applied, then the event failed.
            return application("failed", "apply", [
                {
                    section: "tenant-configuration",
                    members: ["tenant_platform-t1.json"],
                    status: {
                        state: "applied",
                        submission: {taskId: "task-tc", detail: {}},
                        reconciled: false,
                    },
                    history: history(true, "task-tc"),
                },
                {
                    section: "election-event",
                    members: ["event.zip"],
                    status: {state: "failed", error: "import failed", task_id: "task-ev"},
                    history: history(false, "task-ev"),
                },
            ])
        case "refused":
            // Refused before this run applied anything, but an earlier run had
            // applied the tenant configuration, which stays applied.
            return application("refused", "apply", [
                {
                    section: "tenant-configuration",
                    members: ["tenant_platform-t1.json"],
                    status: {state: "unchanged", preview: preview(false)},
                    history: history(true, "task-earlier"),
                },
                {
                    section: "election-event",
                    members: ["event.zip"],
                    status: {state: "blocked", preview: preview(true, ["census differs"])},
                    history: history(false),
                },
            ])
        case "unknown":
            return application("unknown", "apply", [
                {
                    section: "tenant-configuration",
                    members: ["tenant_platform-t1.json"],
                    status: {
                        state: "unknown",
                        error: "an earlier submission is still running",
                        task_id: "task-run",
                    },
                    history: {
                        submissions: [{taskId: "task-run", detail: null}],
                        inFlight: "task-run",
                        applied: false,
                    },
                },
            ])
        default:
            return application(outcome, "preview", [
                {
                    section: "tenant-configuration",
                    members: ["tenant_platform-t1.json"],
                    status: {
                        state: blockers.length ? "blocked" : "previewed",
                        preview: preview(true, blockers),
                    },
                    history: history(false),
                },
                {
                    section: "election-event",
                    members: ["event.zip"],
                    status: {state: "previewed", preview: preview(false)},
                    history: history(false),
                },
            ])
    }
}
