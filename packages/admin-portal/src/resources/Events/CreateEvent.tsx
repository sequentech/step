// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {FC, useMemo, useState, useEffect} from "react"
import {SxProps} from "@mui/material"
import {
    AutocompleteInput,
    Identifier,
    ReferenceInput,
    Create,
    DateTimeInput,
    SimpleForm,
    useGetList,
    useNotify,
    useRefresh,
    useUpdate,
    SelectInput,
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
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

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

interface SelectElectionProps {
    tenantId: string | null
    electionEventId: string | Identifier | undefined
    source: string
    label?: string
    onSelectElection?: (...event: any[]) => void
    customStyle?: SxProps
}

const SelectElection = ({
    tenantId,
    electionEventId,
    source,
    label,
    onSelectElection,
    customStyle,
}: SelectElectionProps) => {
    const aliasRenderer = useAliasRenderer()
    const electionFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {name: searchText.trim()}
    }

    return (
        <ReferenceInput
            required
            fullWidth={true}
            reference="sequent_backend_scheduled_event"
            source={source}
            filter={{
                tenant_id: tenantId,
                election_event_id: electionEventId,
            }}
            perPage={100} // // Setting initial larger records size of areas
            enableGetChoices={({q}) => q && q.length >= 3}
            label={label}
        >
            <AutocompleteInput
                label={label}
                fullWidth={true}
                optionText={aliasRenderer}
                filterToQuery={electionFilterToQuery}
                onChange={onSelectElection}
                debounce={100}
                sx={customStyle}
            />
        </ReferenceInput>
    )
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

    console.log("eventList: ", eventList)
    console.log("selectedEventId: ", selectedEventId)
    console.log("isEditEvent: ", isEditEvent)
    const selectedEvent = useMemo(() => {
        return eventList?.find((event) => event.id === selectedEventId)
    }, [eventList, selectedEventId])

    console.log("is found selectedEvent?: ", selectedEvent)
    console.log("elections: ", elections)
    const [electionId, setElectionId] = useState(null)
    const [electionName, setElectionName] = useState("")

    useEffect(() => {
        console.log("useEffect ", isEditEvent)

        if(isEditEvent) {
            const element = elections?.find((election) => election.id === selectedEvent?.event_payload.election_id)
            const el_id = element?.id
            const name = element?.name?? ""
            console.log("element: ", element)
            console.log("el_id: ", el_id)
            console.log("name: ", name)
            setElectionId(el_id)
            setElectionName(name)
        }
    }, [isEditEvent, selectedEvent])

    console.log("electionId: ", electionId)
    
    interface EnumChoice<T> {
        id: T
        name: string
    }
    const electionChoices = (): Array<EnumChoice<string>> => {
        if (!elections) {
            return []
        }
        const election_choices = elections.map((entry) => {
            console.log("id: ", entry.id)
            console.log("name: ", entry.name)
            return {
                id: entry.id,
                name: entry.name,
            }
        })
        console.log("election_choices: ", election_choices)
        return election_choices
    }
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
                        event_payload: {election_id: electionId},
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
                        eventPayload: {election_id: electionId},
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
                    <InputLabel id="event-type-select-label">
                        {t("eventsScreen.eventType.label")}
                    </InputLabel>
                    <Select
                        required
                        name="event_type"
                        defaultValue={isEditEvent && EventProcessors.START_ELECTION}
                        labelId="event-type-select-label"
                        label={t("eventsScreen.eventType.label")}
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
                    {/* <InputLabel id="election-select-label">
                        {t("common.resources.election")}
                    </InputLabel> */}
                    <SelectInput
                        // name="Election"
                        defaultValue={electionName?? ""}
                        choices={electionChoices()}
                        // electionEventId={electionEventId}
                        // onSelectElection={(election) => setElectionId(election ??  null)}
                        // source={selectedEvent?.event_payload.election_id?? ""}
                        // source={electionName?? ""}
                        // source="eventPayload.election_id"
                        source="eventPayload.election_name"
                        emptyText={t("All the elections")} // TODO
                        // source="event_payload.election_id"
                        // source="id"
                    />
                </FormControl>
                <DateTimeInput
                    required
                    disabled={isLoading}
                    source="dates.start_date"
                    label={t("electionScreen.field.startDateTime")}
                    defaultValue={
                        isEditEvent ? selectedEvent?.cron_config.scheduled_date : scheduleDate
                    }
                    value={isEditEvent ? selectedEvent?.cron_config.scheduled_date : scheduleDate}
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
