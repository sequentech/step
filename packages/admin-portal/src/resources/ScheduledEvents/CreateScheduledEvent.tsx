// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {FC, useEffect, useState} from "react"
import {
    Create,
    DateTimeInput,
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
    Sequent_Backend_Scheduled_Event,
} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {IPermissions} from "@/types/keycloak"
import {ICronConfig, IManageElectionDatePayload} from "@/types/scheduledEvents"
import {VotingStatusChannel} from "@sequentech/ui-core"
import SelectElection from "@/components/election/SelectElection"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"

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
}

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
    const [scheduleDate, setScheduleDate] = useState<string | undefined>(
        isEditEvent ? selectedEvent?.cron_config?.scheduled_date : undefined
    )
    const [eventType, setEventType] = useState<EventProcessors>(
        isEditEvent
            ? ((selectedEvent?.event_processor as EventProcessors | null) ??
                  EventProcessors.START_VOTING_PERIOD)
            : EventProcessors.START_VOTING_PERIOD
    )
    useEffect(() => {
        if (!isEditEvent || !selectedEvent) return
        setEventType(selectedEvent.event_processor as EventProcessors)
        setScheduleDate((selectedEvent.cron_config as ICronConfig)?.scheduled_date)
        const payload = selectedEvent.event_payload as IManageElectionDatePayload
        setElectionId(payload?.election_id ?? null)
        setVotingChannels(
            payload?.voting_channels?.length
                ? payload.voting_channels
                : [VotingStatusChannel.Online, VotingStatusChannel.Kiosk]
        )
    }, [isEditEvent, selectedEvent])
    const isVotingEvent =
        eventType === EventProcessors.START_VOTING_PERIOD ||
        eventType === EventProcessors.END_VOTING_PERIOD
    // Early voting cannot start once online voting has started.
    const opensOnlineWithEarlyVoting =
        eventType === EventProcessors.START_VOTING_PERIOD &&
        votingChannels.includes(VotingStatusChannel.Online) &&
        votingChannels.includes(VotingStatusChannel.EarlyVoting)
    const targetsElection = (event_processor: EventProcessors) => {
        switch (event_processor) {
            case EventProcessors.ALLOW_INIT_REPORT:
            case EventProcessors.START_VOTING_PERIOD:
            case EventProcessors.END_VOTING_PERIOD:
            case EventProcessors.ALLOW_VOTING_PERIOD_END:
            case EventProcessors.ALLOW_TALLY:
                return true
            case EventProcessors.START_ENROLLMENT_PERIOD:
            case EventProcessors.END_ENROLLMENT_PERIOD:
            case EventProcessors.START_LOCKDOWN_PERIOD:
            case EventProcessors.END_LOCKDOWN_PERIOD:
                return false
        }
    }

    const onSubmit = async () => {
        if (opensOnlineWithEarlyVoting) {
            return
        }
        setIsLoading(true)
        try {
            let variables: ManageElectionDatesMutationVariables = {
                electionEventId: electionEventId,
                electionId:
                    targetsElection(eventType as EventProcessors) &&
                    electionId &&
                    electionId.length > 0
                        ? electionId
                        : null,
                scheduledDate: scheduleDate,
                eventProcessor: eventType,
                votingChannels: isVotingEvent ? votingChannels : undefined,
            }
            const {data, errors} = await manageElectionDates({
                variables,
            })
            setIsLoading(false)
            setIsOpenDrawer(false)
            refresh()
            if (data?.manage_election_dates?.error_msg || errors) {
                notify(t("eventsScreen.messages.createError"), {type: "error"})
            } else {
                notify(t("eventsScreen.messages.editSuccess"), {type: "success"})
            }
        } catch (error) {
            setIsLoading(false)
            notify(getGraphQLActionErrorReason(error) ?? t("eventsScreen.messages.createError"), {
                type: "error",
            })
        }
    }
    const isRequiredElection = (eventType: EventProcessors) =>
        ![EventProcessors.START_VOTING_PERIOD, EventProcessors.END_VOTING_PERIOD].includes(
            eventType
        )

    const userTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone

    return (
        <Create hasEdit={isEditEvent}>
            <SimpleForm
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton disabled={opensOnlineWithEarlyVoting} />
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
                        <MenuItem value={EventProcessors.ALLOW_INIT_REPORT}>
                            {t("eventsScreen.eventType.ALLOW_INIT_REPORT")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.START_VOTING_PERIOD}>
                            {t("eventsScreen.eventType.START_VOTING_PERIOD")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.END_VOTING_PERIOD}>
                            {t("eventsScreen.eventType.END_VOTING_PERIOD")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.ALLOW_VOTING_PERIOD_END}>
                            {t("eventsScreen.eventType.ALLOW_VOTING_PERIOD_END")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.START_ENROLLMENT_PERIOD}>
                            {t("eventsScreen.eventType.START_ENROLLMENT_PERIOD")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.END_ENROLLMENT_PERIOD}>
                            {t("eventsScreen.eventType.END_ENROLLMENT_PERIOD")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.START_LOCKDOWN_PERIOD}>
                            {t("eventsScreen.eventType.START_LOCKDOWN_PERIOD")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.END_LOCKDOWN_PERIOD}>
                            {t("eventsScreen.eventType.END_LOCKDOWN_PERIOD")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.ALLOW_TALLY}>
                            {t("eventsScreen.eventType.ALLOW_TALLY")}
                        </MenuItem>
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
                        targetsElection(eventType as EventProcessors) && (
                            <SelectElection
                                isRequired={isRequiredElection(eventType as EventProcessors)}
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
                <DateTimeInput
                    required
                    disabled={isLoading}
                    source="cron_config.scheduled_date"
                    label={
                        eventType === EventProcessors.START_VOTING_PERIOD
                            ? t("electionScreen.field.startDateTimeWithTimezone", {
                                  timezone: userTimeZone,
                              })
                            : t("electionScreen.field.endDateTimeWithTimezone", {
                                  timezone: userTimeZone,
                              })
                    }
                    defaultValue={
                        isEditEvent
                            ? (selectedEvent?.cron_config as ICronConfig | undefined)
                                  ?.scheduled_date
                            : scheduleDate
                    }
                    value={
                        isEditEvent
                            ? (selectedEvent?.cron_config as ICronConfig | undefined)
                                  ?.scheduled_date
                            : scheduleDate
                    }
                    parse={(value) => value && new Date(value).toISOString()}
                    onChange={(value) => {
                        setScheduleDate(
                            value && value.target.value !== ""
                                ? new Date(value.target.value).toISOString()
                                : undefined
                        )
                    }}
                />
                {isLoading ? <CircularProgress /> : null}
            </SimpleForm>
        </Create>
    )
}

export default CreateEvent
