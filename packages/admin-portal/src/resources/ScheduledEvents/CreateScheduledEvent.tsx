// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {FC, useEffect, useState} from "react"
import {
    Create,
    SaveButton,
    SimpleForm,
    Toolbar,
    useGetOne,
    useNotify,
    useRefresh,
} from "react-admin"
import {useFormContext} from "react-hook-form"
import {useTranslation} from "react-i18next"
import {
    CircularProgress,
    Checkbox,
    FormControlLabel,
    FormGroup,
    FormHelperText,
    FormLabel,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    TextField,
    Typography,
} from "@mui/material"
import {useMutation} from "@apollo/client"
import {
    ManageElectionDatesMutation,
    ManageElectionDatesMutationVariables,
    Sequent_Backend_Election,
    Sequent_Backend_Scheduled_Event,
} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {IPermissions} from "@/types/keycloak"
import {ICronConfig, IManageElectionDatePayload} from "@/types/scheduledEvents"
import {VotingStatusChannel} from "@sequentech/ui-core"
import SelectElection from "@/components/election/SelectElection"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {
    ZonedDateTimeField,
    editableOf,
    zonedValueOf,
} from "@/components/timezones/ZonedDateTimeInput"
import {useTimeZoneContext} from "@/components/timezones/useTimeZoneContext"
import {
    formatWallTime,
    useTimeZoneService,
    type IAdminTimeZones,
} from "@/components/timezones/timeZoneService"
import type {IScheduleWarning} from "@/types/lifecycle"
import {OutcomeChangeNotice} from "./OutcomeChangeNotice"

/** The time as entered goes with the instant (VOTE-LIFECYCLE design §5). */
type IManageElectionDatesVariables = ManageElectionDatesMutationVariables & {
    localDateTime?: string | null
    timeZone?: string | null
}

/**
 * A save warning's parameters in words: wall times (`…_local`, "YYYY-MM-DD HH:MM")
 * in the admin format, and the zone (`time_zone`) as its label.
 */
export const warningParams = (
    warning: IScheduleWarning,
    service: Pick<IAdminTimeZones, "text" | "zoneLabel">,
    electionName: (electionId: string) => string
): Record<string, unknown> => ({
    // The Post the warning is about.
    election: warning.election_id ? electionName(warning.election_id) : "",
    ...Object.fromEntries(
        Object.entries(warning.params ?? {}).map(([name, value]) => [
            name,
            typeof value !== "string"
                ? value
                : name.endsWith("_local")
                  ? formatWallTime(value.replace(" ", "T"), service.text)
                  : name === "time_zone"
                    ? service.zoneLabel(value, service.text)
                    : value,
        ])
    ),
})

/** Warnings the form already showed before saving. */
const DST_WARNINGS = ["dst-gap", "dst-overlap"]

interface IManageElectionDatesOutput {
    error_msg?: string | null
    scheduled_date?: string | null
    warnings?: Array<IScheduleWarning> | null
}

interface CreateEventProps {
    electionEventId: string
    setIsOpenDrawer: (state: boolean) => void
    isEditEvent?: boolean
    selectedEventId?: string
    getElectionName: (scheduledEvent: Sequent_Backend_Scheduled_Event) => string
}

export enum EventProcessors {
    ALLOW_INIT_REPORT = "ALLOW_INIT_REPORT",
    START_VOTING_PERIOD = "START_VOTING_PERIOD",
    END_VOTING_PERIOD = "END_VOTING_PERIOD",
    ALLOW_VOTING_PERIOD_END = "ALLOW_VOTING_PERIOD_END",
    START_ENROLLMENT_PERIOD = "START_ENROLLMENT_PERIOD",
    END_ENROLLMENT_PERIOD = "END_ENROLLMENT_PERIOD",
    START_LOCKDOWN_PERIOD = "START_LOCKDOWN_PERIOD",
    END_LOCKDOWN_PERIOD = "END_LOCKDOWN_PERIOD",
    ALLOW_TALLY = "ALLOW_TALLY",
    START_READINESS_TEST = "START_READINESS_TEST",
    END_READINESS_TEST = "END_READINESS_TEST",
    START_FINAL_TESTING = "START_FINAL_TESTING",
    END_FINAL_TESTING = "END_FINAL_TESTING",
    START_TEST_VOTING = "START_TEST_VOTING",
    END_TEST_VOTING = "END_TEST_VOTING",
}

/**
 * Whether a scheduled event of this type can target one election. Enrollment
 * opens per election and still closes for the whole event.
 */
export const targetsElection = (eventProcessor: EventProcessors): boolean => {
    switch (eventProcessor) {
        case EventProcessors.ALLOW_INIT_REPORT:
        case EventProcessors.START_VOTING_PERIOD:
        case EventProcessors.END_VOTING_PERIOD:
        case EventProcessors.ALLOW_VOTING_PERIOD_END:
        case EventProcessors.ALLOW_TALLY:
        case EventProcessors.START_ENROLLMENT_PERIOD:
        case EventProcessors.START_READINESS_TEST:
        case EventProcessors.END_READINESS_TEST:
        case EventProcessors.START_FINAL_TESTING:
        case EventProcessors.END_FINAL_TESTING:
        case EventProcessors.START_TEST_VOTING:
        case EventProcessors.END_TEST_VOTING:
            return true
        case EventProcessors.END_ENROLLMENT_PERIOD:
        case EventProcessors.START_LOCKDOWN_PERIOD:
        case EventProcessors.END_LOCKDOWN_PERIOD:
            return false
    }
}

/** Whether a scheduled event of this type must target one election (otherwise it may be event-wide). */
export const requiresElection = (eventProcessor: EventProcessors): boolean =>
    targetsElection(eventProcessor) &&
    ![
        EventProcessors.START_VOTING_PERIOD,
        EventProcessors.END_VOTING_PERIOD,
        EventProcessors.START_ENROLLMENT_PERIOD,
    ].includes(eventProcessor)

/** Opening and closing voting: the transitions with a predicted outcome (design §5c). */
export const isVotingTransition = (eventProcessor: string | null | undefined): boolean =>
    eventProcessor === EventProcessors.START_VOTING_PERIOD ||
    eventProcessor === EventProcessors.END_VOTING_PERIOD

const VotingChannelsInput: FC<{
    value: VotingStatusChannel[]
    onChange: (channels: VotingStatusChannel[]) => void
    disabled: boolean
    error?: string
}> = ({value, onChange, disabled, error}) => {
    const {t} = useTranslation()
    const {setValue} = useFormContext()
    return (
        <FormControl component="fieldset" margin="normal" error={!!error}>
            <FormLabel component="legend">{t("electionScreen.field.votingChannels")}</FormLabel>
            <FormGroup row>
                {Object.values(VotingStatusChannel).map((channel) => (
                    <FormControlLabel
                        key={channel}
                        label={t(`common.channel.${channel.toLowerCase()}`)}
                        control={
                            <Checkbox
                                checked={value.includes(channel)}
                                disabled={
                                    disabled || (value.length === 1 && value.includes(channel))
                                }
                                onChange={(_, checked) => {
                                    const next = checked
                                        ? [...value, channel]
                                        : value.filter((selected) => selected !== channel)
                                    setValue("event_payload.voting_channels", next, {
                                        shouldDirty: true,
                                    })
                                    onChange(next)
                                }}
                            />
                        }
                    />
                ))}
            </FormGroup>
            {error && <FormHelperText>{error}</FormHelperText>}
        </FormControl>
    )
}

const CreateEvent: FC<CreateEventProps> = ({
    electionEventId,
    setIsOpenDrawer,
    isEditEvent,
    selectedEventId,
    getElectionName,
}) => {
    const {t} = useTranslation()
    const [isLoading, setIsLoading] = useState(false)
    const [votingChannels, setVotingChannels] = useState<VotingStatusChannel[]>([
        VotingStatusChannel.Online,
        VotingStatusChannel.Kiosk,
    ])
    const refresh = useRefresh()
    const [tenantId] = useTenantStore()
    const {data: selectedEvent} = useGetOne<Sequent_Backend_Scheduled_Event>(
        "sequent_backend_scheduled_event",
        {id: selectedEventId},
        {enabled: !!selectedEventId}
    )
    const notify = useNotify()
    const [manageElectionDates] = useMutation<ManageElectionDatesMutation>(MANAGE_ELECTION_DATES, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE,
            },
        },
    })
    const [electionId, setElectionId] = useState<string | null>(
        isEditEvent ? selectedEvent?.event_payload?.election_id : null
    )
    const aliasRenderer = useAliasRenderer()
    const {data: election} = useGetOne<Sequent_Backend_Election>(
        "sequent_backend_election",
        {id: electionId},
        {enabled: !!electionId}
    )
    const electionName = election ? aliasRenderer(election) : null
    const zones = useTimeZoneContext(electionEventId)
    const timeZones = useTimeZoneService()
    // The wall time and zone as edited; the zone follows the election until one is chosen.
    const [edited, setEdited] = useState<{local: string; timezone: string}>({
        local: "",
        timezone: zones.zoneOf(null),
    })
    const [zoneChosen, setZoneChosen] = useState(false)
    // Whether the time was edited here; until then an edited row shows what is stored.
    const [touched, setTouched] = useState(false)
    const stored = isEditEvent ? (selectedEvent?.cron_config as ICronConfig | undefined) : undefined
    // An edited row whose time wasn't touched is saved as stored (seconds and all),
    // so saving its channels doesn't change its fingerprint.
    const zoned =
        stored?.scheduled_date && !touched && edited.local
            ? {
                  scheduled_date: stored.scheduled_date,
                  local: stored.local ?? edited.local,
                  timezone: stored.timezone ?? edited.timezone,
              }
            : zonedValueOf(edited.local, edited.timezone, timeZones)
    // Only the event's configured zones can be saved.
    const unconfiguredZone = !!zoned && !zones.configured.includes(zoned.timezone)
    const [eventType, setEventType] = useState<EventProcessors>(
        isEditEvent
            ? ((selectedEvent?.event_processor as EventProcessors | null) ??
                  EventProcessors.START_VOTING_PERIOD)
            : EventProcessors.START_VOTING_PERIOD
    )
    useEffect(() => {
        if (!isEditEvent || !selectedEvent) return
        setEventType(selectedEvent.event_processor as EventProcessors)
        const payload = selectedEvent.event_payload as IManageElectionDatePayload
        setElectionId(payload?.election_id ?? null)
        setVotingChannels(
            payload?.voting_channels?.length
                ? payload.voting_channels
                : [VotingStatusChannel.Online, VotingStatusChannel.Kiosk]
        )
    }, [isEditEvent, selectedEvent])
    // An edited row shows its time as entered, or an older instant in the row's zone,
    // which is known once the event's zones load; the person's edits are kept.
    useEffect(() => {
        if (!isEditEvent || !selectedEvent || touched) return
        const payload = selectedEvent.event_payload as IManageElectionDatePayload
        const stored = selectedEvent.cron_config as ICronConfig | undefined
        setEdited(editableOf(stored, zones.zoneOf(payload?.election_id), timeZones))
    }, [isEditEvent, selectedEvent, touched, zones, timeZones])
    // A new row's time is in its election's zone (else the primary) until a zone is chosen.
    useEffect(() => {
        if (isEditEvent || zoneChosen) return
        const zone = zones.zoneOf(electionId)
        setEdited((current) => (current.timezone === zone ? current : {...current, timezone: zone}))
    }, [isEditEvent, zoneChosen, zones, electionId])
    const placeName =
        isEditEvent && selectedEvent
            ? getElectionName(selectedEvent)
            : electionId && targetsElection(eventType)
              ? (electionName ?? undefined)
              : t("lifecycle.schedule.allElections")
    const isVotingEvent =
        eventType === EventProcessors.START_VOTING_PERIOD ||
        eventType === EventProcessors.END_VOTING_PERIOD
    // Early voting cannot start once online voting has started.
    const opensOnlineWithEarlyVoting =
        eventType === EventProcessors.START_VOTING_PERIOD &&
        votingChannels.includes(VotingStatusChannel.Online) &&
        votingChannels.includes(VotingStatusChannel.EarlyVoting)
    const onSubmit = async () => {
        if (opensOnlineWithEarlyVoting || !zoned || unconfiguredZone) {
            return
        }
        setIsLoading(true)
        try {
            const variables: IManageElectionDatesVariables = {
                electionEventId: electionEventId,
                electionId:
                    targetsElection(eventType) && electionId && electionId.length > 0
                        ? electionId
                        : null,
                scheduledDate: zoned?.scheduled_date,
                localDateTime:
                    stored?.scheduled_date && !touched ? (stored.local ?? null) : zoned?.local,
                timeZone:
                    stored?.scheduled_date && !touched
                        ? (stored.timezone ?? null)
                        : zoned?.timezone,
                eventProcessor: eventType,
                votingChannels: isVotingEvent ? votingChannels : undefined,
            }
            const {data, errors} = await manageElectionDates({
                variables,
            })
            setIsLoading(false)
            if (data?.manage_election_dates?.error_msg || errors) {
                notify(t("eventsScreen.messages.createError"), {type: "error"})
            } else {
                setIsOpenDrawer(false)
                refresh()
                const saved = t(
                    isEditEvent
                        ? "eventsScreen.messages.editSuccess"
                        : "eventsScreen.messages.createSuccess"
                )
                // Warnings (30-day rule, final testing lead time) never block a save. The
                // DST notes were explained before saving.
                const warnings = (
                    data?.manage_election_dates as IManageElectionDatesOutput | null
                )?.warnings?.filter(({code}) => !DST_WARNINGS.includes(code))
                notify(
                    warnings?.length
                        ? [
                              saved,
                              ...warnings.map((warning) =>
                                  t(
                                      warning.message_key,
                                      warningParams(warning, timeZones, (id) =>
                                          getElectionName({
                                              event_payload: {election_id: id},
                                          } as Sequent_Backend_Scheduled_Event)
                                      )
                                  )
                              ),
                          ].join(" ")
                        : saved,
                    {type: warnings?.length ? "warning" : "success", multiLine: !!warnings?.length}
                )
            }
        } catch (error) {
            setIsLoading(false)
            notify(getGraphQLActionErrorReason(error) ?? t("eventsScreen.messages.createError"), {
                type: "error",
            })
        }
    }
    return (
        <Create hasEdit={isEditEvent}>
            <SimpleForm
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton
                            disabled={opensOnlineWithEarlyVoting || !zoned || unconfiguredZone}
                            alwaysEnable={!!zoned}
                        />
                    </Toolbar>
                }
            >
                <Typography variant="h4">
                    {t(`${isEditEvent ? "eventsScreen.edit.title" : "eventsScreen.create.title"}`)}
                </Typography>
                <Typography variant="body2">
                    {t(
                        `${
                            isEditEvent
                                ? "eventsScreen.edit.subtitle"
                                : "eventsScreen.create.subtitle"
                        }`
                    )}
                </Typography>
                <FormControl fullWidth>
                    <InputLabel id="event-type-select-label">
                        {t("eventsScreen.eventType.label")}
                    </InputLabel>
                    <Select
                        required
                        name="event_type"
                        labelId="event-type-select-label"
                        label={String(t("eventsScreen.eventType.label"))}
                        value={eventType}
                        onChange={(e: any) => setEventType(e.target.value)}
                        disabled={isEditEvent || isLoading}
                    >
                        {Object.values(EventProcessors).map((processor) => (
                            <MenuItem key={processor} value={processor}>
                                {t(`eventsScreen.eventType.${processor}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <FormControl fullWidth>
                    {isEditEvent ? (
                        <TextField
                            label={String(t("eventsScreen.election.label"))}
                            disabled={true}
                            value={selectedEvent ? getElectionName(selectedEvent) : "-"}
                        />
                    ) : (
                        targetsElection(eventType) && (
                            <SelectElection
                                isRequired={requiresElection(eventType)}
                                tenantId={tenantId}
                                electionEventId={electionEventId}
                                label={String(t("eventsScreen.election.label"))}
                                onSelectElection={(electionId) => setElectionId(electionId)}
                                source="event_payload.election_id"
                                disabled={isEditEvent || isLoading}
                                value={electionId}
                            />
                        )
                    )}
                </FormControl>
                {isVotingEvent && (
                    <VotingChannelsInput
                        value={votingChannels}
                        onChange={setVotingChannels}
                        disabled={isLoading}
                        error={
                            opensOnlineWithEarlyVoting
                                ? t("eventsScreen.messages.onlineWithEarlyVoting")
                                : undefined
                        }
                    />
                )}
                <ZonedDateTimeField
                    required
                    disabled={isLoading}
                    label={t("lifecycle.input.scheduledAt")}
                    local={edited.local}
                    timezone={edited.timezone}
                    zones={zones.configured}
                    primary={zones.primary}
                    place={placeName}
                    error={unconfiguredZone}
                    helperText={
                        unconfiguredZone
                            ? t("lifecycle.input.unconfiguredZone", {zone: edited.timezone})
                            : undefined
                    }
                    onChange={(_value, next) => {
                        if (next.timezone !== edited.timezone) setZoneChosen(true)
                        setTouched(true)
                        setEdited(next)
                    }}
                />
                {isVotingTransition(eventType) && zoned ? (
                    <OutcomeChangeNotice
                        electionEventId={electionEventId}
                        change={{
                            id: selectedEventId ?? null,
                            event_processor: eventType,
                            cron_config: zoned,
                            event_payload: {
                                election_id: targetsElection(eventType) ? electionId : null,
                                voting_channels: votingChannels,
                            },
                        }}
                        zone={edited.timezone}
                    />
                ) : null}
                {isLoading ? <CircularProgress /> : null}
            </SimpleForm>
        </Create>
    )
}

export default CreateEvent
