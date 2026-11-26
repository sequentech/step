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

/** The worker saves a refused preview, including absent coverage, on a FAILED task. */
export const refusedPreview = (
    blockers: string[],
    {missingTrustees = false, missingCensus = false} = {}
) => {
    const {application: base} = record("previewed", blockers)
    return {
        application: {
            ...base,
            outcome: "refused",
            coverage: {
                ...base.coverage,
                members: base.coverage.members.filter(
                    (member) => !missingCensus || member.path !== "export_voters-e1.csv"
                ),
                sections: {
                    ...base.coverage.sections,
                    ...(missingTrustees && {trustees: {state: "missing"}}),
                },
                census: missingCensus ? {state: "missing"} : base.coverage.census,
            },
            sections: [
                ...base.sections,
                ...(missingTrustees
                    ? [
                          {
                              section: "trustees",
                              members: [],
                              status: {state: "missing"},
                              history: history(false),
                          },
                      ]
                    : []),
            ],
        },
    }
}

/** The synthetic signed package of the recovery scenario. */
export const ORIGINAL_SHA = "c0".repeat(32)
/** Everything except Trustees required: Trustees is optional, not excluded. */
export const ORIGINAL_REQUIREMENTS = {
    sections: [
        "tenant-configuration",
        "tenant-settings",
        "role-permissions",
        "admin-accounts",
        "messaging-accounts",
        "election-event",
    ],
    census: "required",
}

const appliedSection = (section: string, taskId: string) => ({
    section,
    members: [`${section}.json`],
    status: {state: "applied", submission: {taskId, detail: {}}, reconciled: false},
    history: history(true, taskId),
})

/**
 * An Apply that stopped at the election event (its default S3 template was
 * missing) after every earlier section applied. Trustees was optional and not
 * in the package. FAILED on the task row, as windmill writes a failed outcome.
 */
export const partialApply = ({
    outcome = "failed",
    inFlight = null as string | null,
    requirements = ORIGINAL_REQUIREMENTS as unknown,
    rosterBinding = undefined as unknown,
    tenantId = "t1",
    sha = ORIGINAL_SHA,
} = {}) => ({
    application: {
        format: "sequent.package-application/1",
        tenantId,
        packageSha256: sha,
        mode: "apply",
        outcome,
        ...(requirements ? {managedRequirements: requirements} : {}),
        ...(rosterBinding ? {managedRosterBinding: rosterBinding} : {}),
        coverage: {members: [], sections: {}, census: {state: "present"}},
        sections: [
            appliedSection("tenant-configuration", "task-s1"),
            appliedSection("tenant-settings", "task-s2"),
            appliedSection("role-permissions", "task-s3"),
            appliedSection("admin-accounts", "task-s4"),
            {
                section: "trustees",
                members: [],
                status: {state: "not-included"},
                history: history(false),
            },
            appliedSection("messaging-accounts", "task-s6"),
            {
                section: "election-event",
                members: ["event.zip"],
                status:
                    outcome === "unknown"
                        ? {state: "unknown", error: "not settled", task_id: "task-s7"}
                        : {state: "failed", error: "default template missing", task_id: "task-s7"},
                history: {
                    submissions: [{taskId: "task-s7", detail: {}}],
                    inFlight,
                    applied: false,
                },
            },
        ],
    },
})

/** The Preview that continues it once the template is repaired: only the event changes. */
export const remainingPreview = (sha = ORIGINAL_SHA) => {
    const {application} = partialApply()
    return {
        application: {
            ...application,
            packageSha256: sha,
            mode: "preview",
            outcome: "previewed",
            sections: application.sections.map((section) =>
                section.section === "trustees"
                    ? section
                    : section.section === "election-event"
                      ? {
                            ...section,
                            status: {
                                state: "previewed",
                                preview: {needsApply: true, changes: [], blockers: []},
                            },
                        }
                      : {
                            ...section,
                            status: {
                                state: "previewed",
                                preview: {needsApply: false, changes: [], blockers: []},
                            },
                        }
            ),
        },
    }
}
