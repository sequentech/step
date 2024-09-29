import React, {FC, useMemo, useState} from "react"
import {
    Create,
    DateTimeInput,
    SimpleForm,
    useGetList,
    useNotify,
    useRefresh,
    useUpdate,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    CircularProgress,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    Typography,
} from "@mui/material"
import {useMutation} from "@apollo/client"
import {CreateEventMutation, Sequent_Backend_Election} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {CREATE_EVENT} from "@/queries/CreateEvent"
import {IPermissions} from "@/types/keycloak"
import {v4 as uuidv4} from "uuid"
import {getAttributeLabel} from "@/services/UserService"

interface CreateEventProps {
    electionEventId: string
    setIsOpenDrawer: (state: boolean) => void
    elections?: Sequent_Backend_Election[]
    isEditEvent?: boolean
    selectedEventId?: string
}

export enum EventProcessors {
    START_ELECTION = "START_ELECTION",
    END_ELECTION = "END_ELECTION",
}

const CreateEvent: FC<CreateEventProps> = ({
    electionEventId,
    setIsOpenDrawer,
    elections,
    isEditEvent,
    selectedEventId,
}) => {
    const {t} = useTranslation()
    const [isLoading, setIsLoading] = useState(false)
    const refresh = useRefresh()
    const [tenantId] = useTenantStore()
    const [update] = useUpdate()
    const {data: eventList} = useGetList("sequent_backend_scheduled_event")
    const notify = useNotify()
    const [createEvent] = useMutation<CreateEventMutation>(CREATE_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.EVENTS_CREATE,
            },
        },
    })

    const selectedEvent = useMemo(() => {
        return eventList?.find((event) => event.id === selectedEventId)
    }, [eventList, selectedEventId])
    const [election, setElection] = useState(
        isEditEvent
            ? elections?.find(
                  (election) => election.id === selectedEvent?.event_payload.election_id
              )?.id
            : null
    )
    const [scheduleDate, setScheduleDate] = useState<string | undefined>(
        isEditEvent ? selectedEvent?.cron_config.scheduled_date : null
    )
    const [eventType, setEventType] = useState<EventProcessors | null>(
        isEditEvent ? selectedEvent?.event_processor : null
    )

    const onSubmit = async (data: any) => {
        setIsLoading(true)
        try {
            if (isEditEvent) {
                update("sequent_backend_scheduled_event", {
                    id: selectedEventId,
                    data: {
                        event_processor: eventType,
                        event_payload: {election_id: election},
                        cron_config: {cron: null, scheduled_date: scheduleDate},
                    },
                })
                notify(t("eventsScreen.messages.editSuccess"), {type: "success"})
                setIsLoading(false)
                setIsOpenDrawer(false)
                refresh()
            } else {
                const {data, errors} = await createEvent({
                    variables: {
                        tenantId: tenantId,
                        electionEventId: electionEventId,
                        eventProcessor: eventType,
                        cronConfig: {cron: null, scheduled_date: scheduleDate},
                        eventPayload: {election_id: election},
                        created_at: new Date().toISOString(),
                        id: uuidv4(),
                    },
                })
                notify(t("eventsScreen.messages.createSuccess"), {type: "success"})
                setIsLoading(false)
                setIsOpenDrawer(false)
                refresh()
                if (errors) {
                    notify(t("eventsScreen.messages.createError"), {type: "error"})
                }
            }
        } catch (error) {
            console.error(error)
        }
    }

    return (
        <Create hasEdit={isEditEvent}>
            <SimpleForm onSubmit={onSubmit}>
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
                    <InputLabel id="event-type-select-label">{t("eventType")}</InputLabel>
                    <Select
                        required
                        name="event_type"
                        defaultValue={isEditEvent && EventProcessors.START_ELECTION}
                        labelId="event-type-select-label"
                        label={t("eventType")}
                        value={eventType || ""}
                        onChange={(e: any) => setEventType(e.target.value)}
                    >
                        <MenuItem value={EventProcessors.START_ELECTION}>
                            {t("Start Election")}
                        </MenuItem>
                        <MenuItem value={EventProcessors.END_ELECTION}>
                            {t("End Election")}
                        </MenuItem>
                    </Select>
                </FormControl>
                <FormControl fullWidth>
                    <InputLabel id="select-label">{getAttributeLabel("Election")}</InputLabel>
                    <Select
                        name={"election"}
                        labelId="select-label"
                        label={getAttributeLabel("Election")}
                        value={election || ""}
                        onChange={(e: any) => setElection(e.target.value)}
                    >
                        {elections?.map((election) => (
                            <MenuItem key={election.id} value={election.id}>
                                {election.name}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <DateTimeInput
                    disabled={isLoading}
                    source="dates.start_date"
                    label={t("electionScreen.field.startDateTime")}
                    defaultValue={
                        isEditEvent ? selectedEvent?.cron_config.scheduled_date : scheduleDate
                    }
                    value={
                        isEditEvent
                            ? new Date(selectedEvent?.cron_config.scheduled_date).toISOString()
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
