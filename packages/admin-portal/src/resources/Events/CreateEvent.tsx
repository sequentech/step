import React, {FC, useState} from "react"
import {Create, DateTimeInput, SimpleForm, useNotify, useRefresh} from "react-admin"
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
}

export enum EventProcessors {
    START_ELECTION = "START_ELECTION",
    END_ELECTION = "END_ELECTION",
}

const CreateEvent: FC<CreateEventProps> = ({electionEventId, setIsOpenDrawer, elections}) => {
    const {t} = useTranslation()
    const [isLoading, setIsLoading] = useState(false)
    const [startDate, setStartDate] = useState<string | undefined>(undefined)
    const refresh = useRefresh()
    const [tenantId] = useTenantStore()
    const [election, setElection] = useState(null)
    const [eventType, setEventType] = useState<EventProcessors | "">(
        EventProcessors.START_ELECTION || ""
    )
    const notify = useNotify()
    const [createEvent] = useMutation<CreateEventMutation>(CREATE_EVENT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ADMIN_USER,
            },
        },
    })

    const onSubmit = async (data: any) => {
        setIsLoading(true)
        try {
            const {data, errors} = await createEvent({
                variables: {
                    tenantId: tenantId,
                    electionEventId: electionEventId,
                    eventProcessor: eventType,
                    cronConfig: {cron: null, scheduled_date: startDate},
                    eventPayload: {election_id: election},
                    created_at: new Date().toISOString(),
                    id: uuidv4(),
                },
            })
            notify(t("eventsScreen.createSuccess"), {type: "success"})
            setIsLoading(false)
            setIsOpenDrawer(false)
            refresh()
            if (errors) {
                notify(t("eventsScreen.createError"), {type: "error"})
            }
        } catch (error) {
            console.error(error)
        }
    }

    return (
        <Create>
            <SimpleForm onSubmit={onSubmit}>
                <Typography variant="h4">{"Create Event"}</Typography>
                <Typography variant="body2">{t("tenantScreen.new.subtitle")}</Typography>
                <FormControl fullWidth>
                    <InputLabel id="event-type-select-label">{t("eventType")}</InputLabel>
                    <Select
                        name="event_type"
                        defaultValue={EventProcessors.START_ELECTION}
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
                        defaultValue=""
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
                    parse={(value) => value && new Date(value).toISOString()}
                    onChange={(value) => {
                        setStartDate(
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
