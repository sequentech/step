// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {SigningAction} from "@/lib/signing/types"
import {
    PREVIEW_SCHEDULED_OUTCOME_CHANGE,
    type IChangeApplies,
    type PreviewScheduledOutcomeChangeData,
} from "@/queries/Lifecycle"

/** The rules scheduled openings and closings follow (design §5b). */
export const SCHEDULED_TRANSITION_ACTIONS: ReadonlyArray<string> = [
    SigningAction.OpenVoting,
    SigningAction.CloseVoting,
]

/** The sentence of how a change applies (`tightens`, `loosens`, `tightens-and-loosens`). */
export const appliesKey = (applies: string | null | undefined): string | null =>
    applies
        ? `scheduledOutcome.applies.${applies.replace(/-([a-z])/g, (_match, letter: string) =>
              letter.toUpperCase()
          )}`
        : null

/**
 * What saving an edited Open or Close voting rule would do to scheduled
 * transitions, as the server classifies it (the words its log uses).
 */
export const useRuleChangePreview = (
    electionEventId: string,
    action: string,
    after: {required: boolean; signatures?: number | null},
    changed: boolean
): IChangeApplies | null => {
    const scheduled = SCHEDULED_TRANSITION_ACTIONS.includes(action)
    const {data} = useQuery<PreviewScheduledOutcomeChangeData>(PREVIEW_SCHEDULED_OUTCOME_CHANGE, {
        variables: {
            electionEventId,
            change: {
                rule: {action, required: after.required, signatures: after.signatures ?? null},
            },
        },
        skip: !scheduled || !changed,
    })
    return scheduled && changed ? (data?.preview_scheduled_outcome_change ?? null) : null
}

/** Before saving a rule: how it applies and how many scheduled transitions change outcome. */
export const RuleChangeNotice: React.FC<{preview: IChangeApplies | null}> = ({preview}) => {
    const {t} = useTranslation()
    const key = preview?.applies_message_key ?? appliesKey(preview?.applies)
    if (!preview || !key) return null
    return (
        <Alert
            severity={preview.applies === "tightens" ? "info" : "warning"}
            data-testid="rule-change-notice"
        >
            <div>{t(key)}</div>
            <div>{t("lifecycle.policies.onSave.outcomes", {count: preview.changes.length})}</div>
        </Alert>
    )
}
