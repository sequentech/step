// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useInput} from "react-admin"
import {
    Alert,
    FormControl,
    FormControlLabel,
    FormLabel,
    Radio,
    RadioGroup,
    Stack,
    Typography,
} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    EInitializationScope,
    EUnsignedScheduledClosePolicy,
    type ILifecyclePolicies,
} from "@sequentech/ui-core"
import {
    GET_LIFECYCLE_SNAPSHOTS,
    newestPerTarget,
    PREVIEW_SCHEDULED_OUTCOME_CHANGE,
    type GetLifecycleSnapshotsData,
    type PreviewScheduledOutcomeChangeData,
} from "@/queries/Lifecycle"
import {policiesOf} from "./lifecyclePolicyChange"

/** The translation key part of each value. */
const SCOPE_KEY: Record<EInitializationScope, string> = {
    [EInitializationScope.POST]: "post",
    [EInitializationScope.EVENT]: "event",
    [EInitializationScope.POST_AND_COUNTRY]: "postAndCountry",
}
const CLOSE_KEY: Record<EUnsignedScheduledClosePolicy, string> = {
    [EUnsignedScheduledClosePolicy.REFUSE]: "refuse",
    [EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM]: "runAsSystem",
}

/**
 * One policy's published value: the value each target's newest publication
 * recorded (the event, and each election published on its own), or how many
 * targets have each value when they differ.
 */
const PublishedValue: React.FC<{
    /** The published value of each target; empty when nothing is published. */
    published: ReadonlyArray<string>
    current: string
    label: (value: string) => string
}> = ({published, current, label}) => {
    const {t} = useTranslation()
    const counts = new Map<string, number>()
    for (const value of published) counts.set(value, (counts.get(value) ?? 0) + 1)
    const changed = published.some((value) => value !== current)
    return (
        <Stack spacing={0.5}>
            <Typography variant="body2" color="text.secondary">
                {published.length === 0
                    ? t("lifecycle.policies.nothingPublished")
                    : counts.size === 1
                      ? t("lifecycle.policies.publishedValue", {value: label(published[0])})
                      : t("lifecycle.policies.publishedPerTarget", {
                            values: Array.from(counts.entries())
                                .map(([value, count]) =>
                                    t("lifecycle.policies.publishedCount", {
                                        value: label(value),
                                        count,
                                    })
                                )
                                .join(", "),
                        })}
            </Typography>
            {changed ? (
                <Typography variant="body2" data-testid="changed-since-published">
                    {t("lifecycle.policies.changedSincePublished")}
                </Typography>
            ) : null}
        </Stack>
    )
}

/**
 * Event > Data > Voting lifecycle: when Posts may open with respect to
 * initialization, and what a scheduled close outside the signed configuration
 * does. Each shows its published (signed) value next to the current one, and
 * before saving, how the change applies and which scheduled transitions it
 * changes (design §5b, §5c).
 */
export const VotingLifecycleSettings: React.FC<{
    electionEventId: string
    /** The policies as saved now. */
    saved: ILifecyclePolicies | null | undefined
    disabled?: boolean
}> = ({electionEventId, saved, disabled}) => {
    const {t} = useTranslation()
    const scope = useInput<EInitializationScope>({
        source: "presentation.lifecycle_policies.initialization_scope",
        defaultValue: EInitializationScope.POST,
    })
    const close = useInput<EUnsignedScheduledClosePolicy>({
        source: "presentation.lifecycle_policies.unsigned_scheduled_close",
        defaultValue: EUnsignedScheduledClosePolicy.REFUSE,
    })
    const current = policiesOf({
        initialization_scope: scope.field.value,
        unsigned_scheduled_close: close.field.value,
    })
    const savedPolicies = policiesOf(saved)

    const {data: snapshotData} = useQuery<GetLifecycleSnapshotsData>(GET_LIFECYCLE_SNAPSHOTS, {
        variables: {electionEventId},
        skip: !electionEventId,
    })
    // The newest published snapshot of each target: the copy the scheduler reads.
    const published = newestPerTarget(snapshotData?.get_lifecycle_snapshots?.snapshots ?? []).map(
        (entry) => policiesOf(entry.snapshot?.policies)
    )

    const pending =
        current.initialization_scope !== savedPolicies.initialization_scope ||
        current.unsigned_scheduled_close !== savedPolicies.unsigned_scheduled_close
    // How the save applies comes from the server, so the screen and the log say the same.
    const {data: previewData} = useQuery<PreviewScheduledOutcomeChangeData>(
        PREVIEW_SCHEDULED_OUTCOME_CHANGE,
        {
            variables: {electionEventId, change: {policies: current}},
            skip: !electionEventId || !pending,
        }
    )
    const preview = previewData?.preview_scheduled_outcome_change
    const changed = preview?.changes ?? []

    const scopeLabel = (value: EInitializationScope) =>
        t(`lifecycle.policies.scope.${SCOPE_KEY[value]}.label`)
    const closeLabel = (value: EUnsignedScheduledClosePolicy) =>
        t(`lifecycle.policies.close.${CLOSE_KEY[value]}.label`)

    return (
        <Stack spacing={3} sx={{width: "100%"}} data-testid="voting-lifecycle">
            <Typography variant="body2" color="text.secondary">
                {t("lifecycle.policies.intro")}
            </Typography>
            <FormControl disabled={disabled}>
                <FormLabel id="lifecycle-initialization-scope">
                    {t("lifecycle.policies.scope.title")}
                </FormLabel>
                <RadioGroup
                    aria-labelledby="lifecycle-initialization-scope"
                    value={current.initialization_scope}
                    onChange={(event) => scope.field.onChange(event.target.value)}
                >
                    {Object.values(EInitializationScope).map((value) => (
                        <FormControlLabel
                            key={value}
                            value={value}
                            control={<Radio />}
                            label={
                                <span>
                                    {scopeLabel(value)}
                                    <Typography
                                        component="span"
                                        variant="body2"
                                        color="text.secondary"
                                        sx={{display: "block"}}
                                    >
                                        {t(`lifecycle.policies.scope.${SCOPE_KEY[value]}.help`)}
                                    </Typography>
                                </span>
                            }
                        />
                    ))}
                </RadioGroup>
                <PublishedValue
                    published={published.map(({initialization_scope}) => initialization_scope)}
                    current={current.initialization_scope}
                    label={(value) => scopeLabel(value as EInitializationScope)}
                />
                {current.initialization_scope !== EInitializationScope.POST ? (
                    <Alert severity="warning" sx={{mt: 1}}>
                        {t(
                            `lifecycle.policies.scope.${SCOPE_KEY[current.initialization_scope]}.warning`
                        )}
                    </Alert>
                ) : null}
            </FormControl>

            <FormControl disabled={disabled}>
                <FormLabel id="lifecycle-unsigned-close">
                    {t("lifecycle.policies.close.title")}
                </FormLabel>
                <Typography variant="body2" color="text.secondary">
                    {t("lifecycle.policies.close.help")}
                </Typography>
                <RadioGroup
                    aria-labelledby="lifecycle-unsigned-close"
                    value={current.unsigned_scheduled_close}
                    onChange={(event) => close.field.onChange(event.target.value)}
                >
                    {Object.values(EUnsignedScheduledClosePolicy).map((value) => (
                        <FormControlLabel
                            key={value}
                            value={value}
                            control={<Radio />}
                            label={
                                <span>
                                    {closeLabel(value)}
                                    <Typography
                                        component="span"
                                        variant="body2"
                                        color="text.secondary"
                                        sx={{display: "block"}}
                                    >
                                        {t(`lifecycle.policies.close.${CLOSE_KEY[value]}.help`)}
                                    </Typography>
                                </span>
                            }
                        />
                    ))}
                </RadioGroup>
                <PublishedValue
                    published={published.map(
                        ({unsigned_scheduled_close}) => unsigned_scheduled_close
                    )}
                    current={current.unsigned_scheduled_close}
                    label={(value) => closeLabel(value as EUnsignedScheduledClosePolicy)}
                />
                {current.unsigned_scheduled_close ===
                EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM ? (
                    <Alert severity="warning" sx={{mt: 1}}>
                        {t("lifecycle.policies.close.runAsSystem.warning")}
                    </Alert>
                ) : null}
            </FormControl>

            {pending && preview ? (
                <Alert
                    severity={preview.applies === "tightens" ? "info" : "warning"}
                    data-testid="lifecycle-save-notice"
                >
                    {preview.applies_message_key ? (
                        <div>{t(preview.applies_message_key)}</div>
                    ) : null}
                    <div>{t("lifecycle.policies.onSave.outcomes", {count: changed.length})}</div>
                </Alert>
            ) : null}
        </Stack>
    )
}
