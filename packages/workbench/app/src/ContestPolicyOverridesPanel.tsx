// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Editor panel for the six vote-validation policies on a single
 * contest. Rendered on the contest detail page (`ContestDetailPage`
 * in WorkbenchInspector.tsx). Backed by the ephemeral
 * {@link import("./policyOverridesStore").default policyOverridesStore}
 * — overrides live in memory only, are excluded from snapshots and
 * checkpoints, and are applied at two boundary points:
 *
 *   - Booth open: inside `applyEligibilitySwap` in
 *     `persistence.ts`, when the operator clicks "Cast vote" on a
 *     voter detail page.
 *   - Tally run: inside `handleRunTally` in `TallyPage.tsx`, when
 *     the operator clicks "Run tally" in the sandbox.
 *
 * The merge direction at both sites is "overlay wins": these
 * overrides supersede whatever the contest's baseline
 * `presentation` carries, and supersede whatever the operator may
 * have typed into the tally page's contest textarea (for the six
 * policy fields only — non-policy edits pass through).
 *
 * The four non-preferential policies are always shown. The
 * preferential-only pair (`duplicated_rank_policy`,
 * `preference_gaps_policy`) is only shown when the contest's
 * `counting_algorithm` is non-Plurality, mirroring the gating in
 * `raw_ballot::decode` which is the only decode path that reaches
 * those checkers.
 */

import type {CSSProperties, ReactElement} from "react"

import {
    EBlankVotePolicy,
    EDuplicatedRankPolicy,
    EInvalidVotePolicy,
    EOverVotePolicy,
    EPreferenceGapsPolicy,
    EUnderVotePolicy,
} from "@sequentech/ui-core"

import {
    BOUNDS_KEYS,
    clearContestOverrides,
    setPolicyOverride,
    useContestPolicyOverlay,
    type ContestPolicyKey,
    type ContestPolicyOverlay,
} from "./policyOverridesStore"
interface PolicyMeta<K extends ContestPolicyKey> {
    key: K
    label: string
    /** Value space, in display order. */
    options: ReadonlyArray<{value: ContestPolicyOverlay[K]; label: string}>
    /** Documented default per FIXTURE_VARIANCE.md §10.A. Shown as the
     *  "(baseline default)" hint when the contest has no explicit
     *  value and no override. */
    defaultLabel: string
    /** When true, this policy is only consulted by `raw_ballot::decode`
     *  (preferential contests) and is hidden on Plurality. */
    preferentialOnly?: boolean
}

const POLICY_META: ReadonlyArray<PolicyMeta<ContestPolicyKey>> = [
    {
        key: "invalid_vote_policy",
        label: "Invalid-vote policy",
        defaultLabel: "allowed",
        options: [
            {value: EInvalidVotePolicy.ALLOWED, label: "allowed (default)"},
            {value: EInvalidVotePolicy.WARN, label: "warn"},
            {
                value: EInvalidVotePolicy.WARN_INVALID_IMPLICIT_AND_EXPLICIT,
                label: "warn-invalid-implicit-and-explicit",
            },
            {value: EInvalidVotePolicy.NOT_ALLOWED, label: "not-allowed"},
            {
                value: EInvalidVotePolicy.ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
                label: "allowed-with-exclusive-explicit",
            },
        ],
    },
    {
        key: "over_vote_policy",
        label: "Over-vote policy",
        defaultLabel: "allowed-with-msg-and-alert",
        options: [
            {value: EOverVotePolicy.ALLOWED, label: "allowed"},
            {
                value: EOverVotePolicy.ALLOWED_WITH_MSG,
                label: "allowed-with-msg",
            },
            {
                value: EOverVotePolicy.ALLOWED_WITH_MSG_AND_ALERT,
                label: "allowed-with-msg-and-alert (default)",
            },
            {
                value: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_ALERT,
                label: "not-allowed-with-msg-and-alert",
            },
            {
                value: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE,
                label: "not-allowed-with-msg-and-disable",
            },
        ],
    },
    {
        key: "under_vote_policy",
        label: "Under-vote policy",
        defaultLabel: "allowed",
        options: [
            {value: EUnderVotePolicy.ALLOWED, label: "allowed (default)"},
            {value: EUnderVotePolicy.WARN, label: "warn"},
            {
                value: EUnderVotePolicy.WARN_ONLY_IN_REVIEW,
                label: "warn-only-in-review",
            },
            {
                value: EUnderVotePolicy.WARN_AND_ALERT,
                label: "warn-and-alert",
            },
        ],
    },
    {
        key: "blank_vote_policy",
        label: "Blank-vote policy",
        defaultLabel: "allowed",
        options: [
            {value: EBlankVotePolicy.ALLOWED, label: "allowed (default)"},
            {value: EBlankVotePolicy.WARN, label: "warn"},
            {
                value: EBlankVotePolicy.WARN_ONLY_IN_REVIEW,
                label: "warn-only-in-review",
            },
            {value: EBlankVotePolicy.NOT_ALLOWED, label: "not-allowed"},
        ],
    },
    {
        key: "duplicated_rank_policy",
        label: "Duplicated-rank policy",
        defaultLabel: "allowed-warn-and-dialog",
        preferentialOnly: true,
        options: [
            {
                value: EDuplicatedRankPolicy.ALLOWED_WARN_AND_DIALOG,
                label: "allowed-warn-and-dialog (default)",
            },
            {
                value: EDuplicatedRankPolicy.NOT_ALLOWED_WARN_AND_DIALOG,
                label: "not-allowed-warn-and-dialog",
            },
        ],
    },
    {
        key: "preference_gaps_policy",
        label: "Preference-gaps policy",
        defaultLabel: "allowed-warn-and-dialog",
        preferentialOnly: true,
        options: [
            {
                value: EPreferenceGapsPolicy.ALLOWED_WARN_AND_DIALOG,
                label: "allowed-warn-and-dialog (default)",
            },
            {
                value: EPreferenceGapsPolicy.NOT_ALLOWED_WARN_AND_DIALOG,
                label: "not-allowed-warn-and-dialog",
            },
        ],
    },
]

/** Shape we read from the contest descriptor. Kept loose because the
 *  workbench treats portal-shaped and velvet-shaped rows
 *  interchangeably. */
interface ContestForPanel {
    id: string
    counting_algorithm?: string | null
    voting_type?: string | null
    presentation?: Record<string, unknown> | null
    /** Contest-level bounds. The policies above are dead code in
     *  zones the bounds make empty (e.g. `blank_vote_policy` is
     *  inert unless `min_votes == 0`), so they're exposed here as
     *  first-class dials alongside the policies. */
    min_votes?: number | null
    max_votes?: number | null
}

export interface ContestPolicyOverridesPanelProps {
    contest: ContestForPanel
}

/** Decide whether the contest's algorithm reaches the preferential-
 *  only checkers (`check_duplicated_rank_policy`,
 *  `check_preference_gaps_policy`). `raw_ballot::decode` runs them
 *  only when `counting_algorithm` is non-Plurality; we mirror that
 *  here by treating anything other than `plurality-at-large` (case-
 *  insensitive, with a small bit of tolerance for the velvet vs
 *  portal serialisations) as preferential. */
function isPreferential(contest: ContestForPanel): boolean {
    const raw =
        (typeof contest.counting_algorithm === "string"
            ? contest.counting_algorithm
            : undefined) ??
        (typeof contest.voting_type === "string"
            ? contest.voting_type
            : undefined)
    if (!raw) return false
    const lower = raw.toLowerCase()
    if (lower === "plurality-at-large") return false
    if (lower === "plurality") return false
    return true
}

export function ContestPolicyOverridesPanel(
    {contest}: ContestPolicyOverridesPanelProps
): ReactElement {
    const overlay = useContestPolicyOverlay(contest.id)
    const overlayCount = Object.keys(overlay).length
    const preferential = isPreferential(contest)
    const visible = POLICY_META.filter(
        (m) => !m.preferentialOnly || preferential
    )

    return (
        <section style={styles.panel}>
            <header style={styles.header}>
                <h2 style={styles.h2}>Policy overrides</h2>
                {overlayCount > 0 ? (
                    <span style={styles.activeBadge}>
                        {overlayCount} active
                    </span>
                ) : null}
                <span style={styles.spacer} />
                {overlayCount > 0 ? (
                    <button
                        type="button"
                        onClick={() => clearContestOverrides(contest.id)}
                        style={styles.clearButton}
                        title="Drop every override on this contest"
                    >
                        Clear all
                    </button>
                ) : null}
            </header>
            <p style={styles.intro}>
                Ephemeral, per-tab overrides for the six vote-
                validation policies plus the two contest bounds
                (<code>min_votes</code>, <code>max_votes</code>) that
                frame them. Applied at booth open (when you click{" "}
                <strong>Cast vote</strong> on a voter page) and at
                tally run (when you click <strong>Run tally</strong>{" "}
                in the sandbox). Not saved with snapshots or
                checkpoints; revert by clicking <em>reset</em> or
                reloading the page.
            </p>
            <BoundsEditor
                contestId={contest.id}
                baselineMin={
                    typeof contest.min_votes === "number"
                        ? contest.min_votes
                        : undefined
                }
                baselineMax={
                    typeof contest.max_votes === "number"
                        ? contest.max_votes
                        : undefined
                }
                overlay={overlay}
            />
            <table style={styles.table}>
                <tbody>
                    {visible.map((meta) => {
                        const baselineRaw = (
                            contest.presentation as
                                | Record<string, unknown>
                                | null
                                | undefined
                        )?.[meta.key]
                        const baseline =
                            typeof baselineRaw === "string"
                                ? baselineRaw
                                : undefined
                        const override = overlay[meta.key]
                        const effective = override ?? baseline
                        return (
                            <PolicyRow
                                key={meta.key}
                                contestId={contest.id}
                                meta={meta}
                                baseline={baseline}
                                override={override}
                                effective={effective}
                            />
                        )
                    })}
                </tbody>
            </table>
            {!preferential ? (
                <p style={styles.footnote}>
                    Preferential-only policies (
                    <code>duplicated_rank_policy</code>,{" "}
                    <code>preference_gaps_policy</code>) are hidden
                    because this contest's counting algorithm is
                    Plurality-at-Large — they would never be reached
                    by the decode path.
                </p>
            ) : null}
        </section>
    )
}

/** Numeric editor for the two contest-level bounds. They live on the
 *  contest, not on `presentation`, so we render them above the
 *  policy table to make the coupling explicit: most policies are
 *  inert outside specific (min, max) ranges (e.g.
 *  `blank_vote_policy` only fires when `min_votes == 0`;
 *  `under_vote_policy` only fires when `max_votes - min_votes >= 2`).
 *  Exposing the bounds keeps the operator out of the "I flipped a
 *  policy and nothing happened" no-man's-land. */
function BoundsEditor({
    contestId,
    baselineMin,
    baselineMax,
    overlay,
}: {
    contestId: string
    baselineMin: number | undefined
    baselineMax: number | undefined
    overlay: ContestPolicyOverlay
}): ReactElement {
    return (
        <div style={styles.boundsRow}>
            {BOUNDS_KEYS.map((key) => {
                const baseline =
                    key === "min_votes" ? baselineMin : baselineMax
                const override = overlay[key] as number | undefined
                const isOverridden = override !== undefined
                const effective = isOverridden ? override : baseline
                return (
                    <label key={key} style={styles.boundsCell}>
                        <span style={styles.boundsLabel}>
                            <code>{key}</code>
                        </span>
                        <input
                            type="number"
                            min={0}
                            step={1}
                            value={effective ?? ""}
                            placeholder={
                                baseline === undefined
                                    ? "(unset)"
                                    : `baseline ${baseline}`
                            }
                            onChange={(e) => {
                                const raw = e.target.value
                                if (raw === "") {
                                    setPolicyOverride(
                                        contestId,
                                        key,
                                        undefined
                                    )
                                    return
                                }
                                const n = Number.parseInt(raw, 10)
                                if (!Number.isFinite(n) || n < 0) return
                                setPolicyOverride(contestId, key, n)
                            }}
                            style={styles.boundsInput}
                            aria-label={`${key} override`}
                        />
                        {isOverridden ? (
                            <button
                                type="button"
                                onClick={() =>
                                    setPolicyOverride(
                                        contestId,
                                        key,
                                        undefined
                                    )
                                }
                                style={styles.resetButton}
                                title="Revert to baseline"
                            >
                                reset
                            </button>
                        ) : (
                            <span style={styles.baselineHint}>baseline</span>
                        )}
                    </label>
                )
            })}
        </div>
    )
}

function PolicyRow<K extends ContestPolicyKey>({
    contestId,
    meta,
    baseline,
    override,
    effective,
}: {
    contestId: string
    meta: PolicyMeta<K>
    baseline: string | undefined
    override: ContestPolicyOverlay[K] | undefined
    effective: string | undefined
}): ReactElement {
    const isOverridden = override !== undefined
    return (
        <tr>
            <th scope="row" style={styles.th}>
                <code>{meta.key}</code>
                <div style={styles.label}>{meta.label}</div>
            </th>
            <td style={styles.td}>
                <select
                    value={effective ?? ""}
                    onChange={(e) => {
                        const v = e.target.value
                        // Empty value (the synthetic "(baseline)"
                        // option) clears the override; any real enum
                        // value sets it.
                        if (v === "") {
                            setPolicyOverride(contestId, meta.key, undefined)
                        } else {
                            setPolicyOverride(
                                contestId,
                                meta.key,
                                v as ContestPolicyOverlay[K]
                            )
                        }
                    }}
                    style={styles.select}
                    aria-label={`${meta.label} override`}
                >
                    {baseline === undefined ? (
                        <option value="">
                            (baseline default — {meta.defaultLabel})
                        </option>
                    ) : (
                        <option value="">
                            (baseline — {baseline})
                        </option>
                    )}
                    {meta.options.map((opt) => (
                        <option
                            key={String(opt.value)}
                            value={String(opt.value)}
                        >
                            {opt.label}
                        </option>
                    ))}
                </select>
            </td>
            <td style={styles.td}>
                {isOverridden ? (
                    <button
                        type="button"
                        onClick={() =>
                            setPolicyOverride(contestId, meta.key, undefined)
                        }
                        style={styles.resetButton}
                        title="Revert to baseline"
                    >
                        reset
                    </button>
                ) : (
                    <span style={styles.baselineHint}>baseline</span>
                )}
            </td>
        </tr>
    )
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles: Record<string, CSSProperties> = {
    panel: {
        marginTop: "1.5rem",
        padding: "0.9rem 1rem",
        border: "1px solid #3a3a3a",
        background: "#2a2a2a",
        borderRadius: 6,
    },
    header: {
        display: "flex",
        alignItems: "baseline",
        gap: "0.6rem",
        marginBottom: "0.25rem",
    },
    h2: {
        fontSize: "1rem",
        margin: 0,
        color: "#e0e0e0",
    },
    spacer: {flex: 1},
    activeBadge: {
        padding: "0.1rem 0.4rem",
        fontSize: "0.72rem",
        background: "#3d3000",
        color: "#f0c200",
        border: "1px solid #f0c200",
        borderRadius: 3,
    },
    clearButton: {
        padding: "0.2rem 0.6rem",
        background: "#383838",
        color: "#e0e0e0",
        border: "1px solid #555",
        borderRadius: 3,
        fontSize: "0.78rem",
        cursor: "pointer",
    },
    intro: {
        margin: "0.25rem 0 0.75rem 0",
        fontSize: "0.82rem",
        color: "#999",
    },
    table: {
        width: "100%",
        borderCollapse: "collapse",
    },
    th: {
        textAlign: "left",
        verticalAlign: "top",
        padding: "0.35rem 0.5rem 0.35rem 0",
        fontWeight: "normal",
        width: "16rem",
    },
    label: {
        fontSize: "0.78rem",
        color: "#999",
    },
    td: {
        padding: "0.35rem 0.5rem",
        verticalAlign: "top",
    },
    select: {
        width: "100%",
        fontSize: "0.85rem",
        padding: "0.25rem 0.4rem",
        background: "#303030",
        color: "#e0e0e0",
        border: "1px solid #4a4a4a",
        borderRadius: 3,
    },
    resetButton: {
        padding: "0.2rem 0.6rem",
        background: "#383838",
        color: "#e0e0e0",
        border: "1px solid #555",
        borderRadius: 3,
        fontSize: "0.78rem",
        cursor: "pointer",
    },
    baselineHint: {
        fontSize: "0.78rem",
        color: "#888",
    },
    footnote: {
        margin: "0.75rem 0 0 0",
        fontSize: "0.78rem",
        color: "#999",
    },
    boundsRow: {
        display: "flex",
        gap: "1rem",
        margin: "0 0 0.85rem 0",
        padding: "0.55rem 0.65rem",
        background: "#252525",
        border: "1px solid #3a3a3a",
        borderRadius: 4,
        flexWrap: "wrap",
    },
    boundsCell: {
        display: "flex",
        alignItems: "center",
        gap: "0.4rem",
        fontSize: "0.82rem",
    },
    boundsLabel: {
        color: "#e0e0e0",
    },
    boundsInput: {
        width: "5.5rem",
        fontSize: "0.85rem",
        padding: "0.2rem 0.35rem",
        background: "#303030",
        color: "#e0e0e0",
        border: "1px solid #4a4a4a",
        borderRadius: 3,
    },
}
