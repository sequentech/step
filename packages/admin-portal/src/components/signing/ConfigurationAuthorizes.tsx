// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {useGetList} from "react-admin"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import type {TFunction} from "i18next"
import {Alert, Box, Stack, Typography} from "@mui/material"
import {
    EInitializationScope,
    EInitializeReportPolicy,
    EUnsignedScheduledClosePolicy,
    type ILifecyclePolicies,
} from "@sequentech/ui-core"
import type {Sequent_Backend_Election} from "@/gql/graphql"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {
    GET_CONFIGURATION_APPROVALS,
    type GetConfigurationApprovalsData,
    type IConfigurationApproval,
} from "@/queries/Lifecycle"
import type {ILifecycleSnapshot, IRuleSnapshot, IScheduledTransition} from "@/types/lifecycle"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"
import {useTimeZoneContext} from "@/components/timezones/useTimeZoneContext"
import {
    EPolicyChange,
    closePolicyChange,
    policiesOf,
    ruleChange,
    scopeChange,
} from "@/components/election-event/lifecyclePolicyChange"

/**
 * The subject fields of an Approve configuration request that hold the
 * lifecycle snapshot (design §5a, §5b). They are shown here, not as raw rows.
 */
export const LIFECYCLE_SUBJECT_KEYS = [
    "schedule",
    "policies",
    "open_voting",
    "close_voting",
    "post_channels",
]

/** The voting channels each Post had when the configuration was approved (election id → channels). */
export const postChannelsOf = (
    subject: Record<string, unknown>
): Array<[string, Array<string>]> => {
    const value = subject.post_channels
    if (!value || typeof value !== "object" || Array.isArray(value)) return []
    return Object.entries(value as Record<string, unknown>).map(([id, channels]) => [
        id,
        Array.isArray(channels) ? channels.map(String) : [],
    ])
}

const SCOPE_KEY: Record<EInitializationScope, string> = {
    [EInitializationScope.POST]: "post",
    [EInitializationScope.EVENT]: "event",
    [EInitializationScope.POST_AND_COUNTRY]: "postAndCountry",
}
const CLOSE_KEY: Record<EUnsignedScheduledClosePolicy, string> = {
    [EUnsignedScheduledClosePolicy.REFUSE]: "refuse",
    [EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM]: "runAsSystem",
}

/** The lifecycle snapshot a configuration subject signs; undefined fields read as absent. */
export const snapshotOfSubject = (subject: Record<string, unknown>): ILifecycleSnapshot => ({
    schedule: Array.isArray(subject.schedule)
        ? (subject.schedule as Array<IScheduledTransition>)
        : [],
    policies: (subject.policies as ILifecyclePolicies | undefined) ?? null,
    open_voting: (subject.open_voting as IRuleSnapshot | undefined) ?? null,
    close_voting: (subject.close_voting as IRuleSnapshot | undefined) ?? null,
    initialization_report_policies: subject.initialization_report_policies as
        | ILifecycleSnapshot["initialization_report_policies"]
        | undefined,
})

/** "Opening needs 2 signatures" / "Opening needs no signatures". */
const ruleText = (t: TFunction, transition: "open" | "close", rule: IRuleSnapshot | null) =>
    rule?.required
        ? t(`lifecycle.authorizes.rule.${transition}Needs`, {count: rule.signatures ?? 1})
        : t(`lifecycle.authorizes.rule.${transition}NoSignatures`)

/** One difference from the previous approved configuration. */
export interface IConfigurationDiff {
    change: EPolicyChange
    setting: string
    before: string
    after: string
}

/** What changed since the previous approved configuration, each as a tightening or a loosening. */
export const configurationDiff = (
    t: TFunction,
    previous: ILifecycleSnapshot,
    next: ILifecycleSnapshot,
    postName: (electionId: string) => string = (electionId) => electionId
): Array<IConfigurationDiff> => {
    const from = policiesOf(previous.policies)
    const to = policiesOf(next.policies)
    const scope = (value: EInitializationScope) =>
        t(`lifecycle.policies.scope.${SCOPE_KEY[value]}.label`)
    const close = (value: EUnsignedScheduledClosePolicy) =>
        t(`lifecycle.policies.close.${CLOSE_KEY[value]}.label`)
    const rule = (value: IRuleSnapshot | null | undefined) =>
        value?.required
            ? t("lifecycle.authorizes.rule.signatures", {count: value.signatures ?? 1})
            : t("lifecycle.authorizes.rule.none")
    const strictness = (value: IRuleSnapshot | null | undefined) => ({
        required: !!value?.required,
        signatures: value?.signatures ?? null,
    })
    const diffs: Array<IConfigurationDiff> = [
        {
            change: scopeChange(from.initialization_scope, to.initialization_scope),
            setting: t("lifecycle.policies.scope.title"),
            before: scope(from.initialization_scope),
            after: scope(to.initialization_scope),
        },
        {
            change: closePolicyChange(from.unsigned_scheduled_close, to.unsigned_scheduled_close),
            setting: t("lifecycle.policies.close.title"),
            before: close(from.unsigned_scheduled_close),
            after: close(to.unsigned_scheduled_close),
        },
        {
            change: ruleChange(strictness(previous.open_voting), strictness(next.open_voting)),
            setting: t("lifecycle.authorizes.rule.openSetting"),
            before: rule(previous.open_voting),
            after: rule(next.open_voting),
        },
        {
            change: ruleChange(strictness(previous.close_voting), strictness(next.close_voting)),
            setting: t("lifecycle.authorizes.rule.closeSetting"),
            before: rule(previous.close_voting),
            after: rule(next.close_voting),
        },
    ]
    const beforeReports = previous.initialization_report_policies ?? {}
    const afterReports = next.initialization_report_policies ?? {}
    const reportPosts = new Set([...Object.keys(beforeReports), ...Object.keys(afterReports)])
    for (const electionId of Array.from(reportPosts).sort()) {
        const before = beforeReports[electionId] ?? EInitializeReportPolicy.NOT_REQUIRED
        const after = afterReports[electionId] ?? EInitializeReportPolicy.NOT_REQUIRED
        if (before === after) continue
        diffs.push({
            change:
                after === EInitializeReportPolicy.REQUIRED
                    ? EPolicyChange.TIGHTENS
                    : EPolicyChange.LOOSENS,
            setting: t("lifecycle.authorizes.reportPolicyOf", {
                election: postName(electionId),
                value: t("electionScreen.initializeReportPolicy.label"),
            }),
            before: t(`electionScreen.initializeReportPolicy.${before}`),
            after: t(`electionScreen.initializeReportPolicy.${after}`),
        })
    }
    return diffs.filter(({change}) => change !== EPolicyChange.NONE)
}

/**
 * The target of a configuration approval: the Post (election id) whose
 * publication it approves, or null for the event. The scope key ends with it.
 */
export const approvalTarget = (scopeKey: string | null | undefined): string | null => {
    const last = (scopeKey ?? "").split("|").pop() ?? ""
    return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(last)
        ? last
        : null
}

/**
 * The previous approved configuration: the newest executed approval (of
 * `approvals`, newest first) whose target applies to this one's target. A
 * Post's own approval or the event's applies to a Post; only the event's to
 * the event.
 */
export const previousApproval = (
    approvals: ReadonlyArray<IConfigurationApproval>,
    target: string | null
): IConfigurationApproval | null =>
    approvals.find((approval) => {
        const other = approvalTarget(approval.scope_key)
        return other === null || other === target
    }) ?? null

/**
 * The configuration approval's "What this approval authorizes" (design §5c):
 * the scheduled openings and closings it covers, in each one's zone, the
 * policy and rule values it signs, and how they differ from the previous
 * approved configuration, each named as a tightening or a loosening.
 */
export const ConfigurationAuthorizes: React.FC<{
    subject: Record<string, unknown>
    electionEventId: string
    /** The approval request, whose target and predecessors are read. */
    requestId: string
}> = ({subject, electionEventId, requestId}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const aliasRenderer = useAliasRenderer()
    const zones = useTimeZoneContext(electionEventId)
    const snapshot = useMemo(() => snapshotOfSubject(subject), [subject])
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId},
    })
    const {data: approvalsData, loading} = useQuery<GetConfigurationApprovalsData>(
        GET_CONFIGURATION_APPROVALS,
        {variables: {electionEventId, requestId}}
    )
    const previous = approvalsData
        ? previousApproval(
              approvalsData.approvals,
              approvalTarget(approvalsData.current?.scope_key)
          )
        : null
    const diffs = previous
        ? configurationDiff(t, snapshotOfSubject(previous.subject), snapshot, (electionId) => {
              const election = elections?.find(({id}) => id === electionId)
              return election ? aliasRenderer(election) : electionId
          })
        : null
    const policies = policiesOf(snapshot.policies)
    const reportPolicies = Object.entries(snapshot.initialization_report_policies ?? {})
    const placeOf = (transition: IScheduledTransition) => {
        if (!transition.election_id) return t("lifecycle.schedule.allElections")
        const election = elections?.find(({id}) => id === transition.election_id)
        return election ? aliasRenderer(election) : transition.election_id
    }

    return (
        <Stack spacing={1.5} data-testid="configuration-authorizes">
            <Typography component="h3" sx={{fontWeight: 600}}>
                {t("lifecycle.authorizes.title")}
            </Typography>
            <Box>
                <Typography variant="body2" sx={{fontWeight: 600}}>
                    {t("lifecycle.authorizes.schedule")}
                </Typography>
                {snapshot.schedule?.length ? (
                    <Box component="ul" sx={{m: 0, pl: 2}}>
                        {snapshot.schedule.map((transition) => {
                            const zone = transition.timezone ?? zones.zoneOf(transition.election_id)
                            const opens = transition.event_processor === "START_VOTING_PERIOD"
                            return (
                                <li key={transition.scheduled_event_id}>
                                    <Typography variant="body2">
                                        {t(
                                            opens
                                                ? "lifecycle.authorizes.opens"
                                                : "lifecycle.authorizes.closes",
                                            {
                                                time: transition.scheduled_date
                                                    ? service.formatPlaceTime(
                                                          transition.scheduled_date,
                                                          zone,
                                                          placeOf(transition),
                                                          service.text
                                                      )
                                                    : placeOf(transition),
                                            }
                                        )}
                                    </Typography>
                                </li>
                            )
                        })}
                    </Box>
                ) : (
                    <Typography variant="body2" color="text.secondary">
                        {t("lifecycle.authorizes.noSchedule")}
                    </Typography>
                )}
            </Box>
            {postChannelsOf(subject).length ? (
                <Box data-testid="authorizes-channels">
                    <Typography variant="body2" sx={{fontWeight: 600}}>
                        {t("lifecycle.authorizes.channels")}
                    </Typography>
                    <Box component="ul" sx={{m: 0, pl: 2}}>
                        {postChannelsOf(subject).map(([electionId, channels]) => (
                            <li key={electionId}>
                                <Typography variant="body2">
                                    {t("lifecycle.authorizes.channelsOf", {
                                        election: placeOf({
                                            election_id: electionId,
                                        } as IScheduledTransition),
                                        channels: channels.length
                                            ? channels
                                                  .map((channel) =>
                                                      t(`common.channel.${channel.toLowerCase()}`, {
                                                          defaultValue: channel,
                                                      })
                                                  )
                                                  .join(", ")
                                            : t("lifecycle.authorizes.noChannels"),
                                    })}
                                </Typography>
                            </li>
                        ))}
                    </Box>
                </Box>
            ) : null}
            <Box>
                <Typography variant="body2" sx={{fontWeight: 600}}>
                    {t("lifecycle.authorizes.settings")}
                </Typography>
                <Box component="ul" sx={{m: 0, pl: 2}}>
                    <li>
                        <Typography variant="body2">
                            {ruleText(t, "open", snapshot.open_voting ?? null)}
                        </Typography>
                    </li>
                    <li>
                        <Typography variant="body2">
                            {ruleText(t, "close", snapshot.close_voting ?? null)}
                        </Typography>
                    </li>
                    <li>
                        <Typography variant="body2">
                            {t("lifecycle.authorizes.unsignedClose", {
                                value: t(
                                    `lifecycle.policies.close.${CLOSE_KEY[policies.unsigned_scheduled_close]}.label`
                                ),
                            })}
                        </Typography>
                    </li>
                    <li>
                        <Typography variant="body2">
                            {t("lifecycle.authorizes.initialization", {
                                value: t(
                                    `lifecycle.policies.scope.${SCOPE_KEY[policies.initialization_scope]}.label`
                                ),
                            })}
                        </Typography>
                    </li>
                </Box>
            </Box>
            {reportPolicies.length ? (
                <Box data-testid="authorizes-initialization-reports">
                    <Typography variant="body2" sx={{fontWeight: 600}}>
                        {t("electionScreen.initializeReportPolicy.label")}
                    </Typography>
                    <Box component="ul" sx={{m: 0, pl: 2}}>
                        {reportPolicies.map(([electionId, policy]) => (
                            <li key={electionId}>
                                <Typography variant="body2">
                                    {t("lifecycle.authorizes.reportPolicyOf", {
                                        election: placeOf({
                                            election_id: electionId,
                                        } as IScheduledTransition),
                                        value: t(`electionScreen.initializeReportPolicy.${policy}`),
                                    })}
                                </Typography>
                            </li>
                        ))}
                    </Box>
                    {reportPolicies.some(
                        ([, policy]) => policy === EInitializeReportPolicy.REQUIRED
                    ) ? (
                        <Typography variant="body2" color="text.secondary">
                            {t("lifecycle.authorizes.initializationRetained")}
                        </Typography>
                    ) : null}
                </Box>
            ) : null}
            {loading ? null : diffs === null ? (
                <Typography variant="body2" color="text.secondary">
                    {t("lifecycle.authorizes.firstConfiguration")}
                </Typography>
            ) : diffs.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                    {t("lifecycle.authorizes.sameAsPrevious")}
                </Typography>
            ) : (
                <Stack spacing={0.5} data-testid="configuration-diff">
                    <Typography variant="body2" color="text.secondary">
                        {t("lifecycle.authorizes.comparedWith", {code: previous?.code})}
                    </Typography>
                    {diffs.map((diff) => (
                        <Alert
                            key={diff.setting}
                            severity={diff.change === EPolicyChange.TIGHTENS ? "info" : "warning"}
                        >
                            {t(`lifecycle.authorizes.diff.${diff.change}`, {
                                setting: diff.setting,
                                before: diff.before,
                                after: diff.after,
                            })}
                        </Alert>
                    ))}
                </Stack>
            )}
        </Stack>
    )
}
