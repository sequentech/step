// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useMutation} from "@apollo/client"
import React, {useEffect, useState} from "react"
import {CreateElectionEventMutation} from "../../gql/graphql"
import {v4} from "uuid"
import {
    BooleanInput,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextInput,
    useGetOne,
    useNotify,
    useRefresh,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {INSERT_ELECTION_EVENT} from "../../queries/InsertElectionEvent"
import {Box, CircularProgress, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {isNull} from "@sequentech/ui-essentials"
import {useNavigate} from "react-router"
import {useTenantStore} from "../../providers/TenantContextProvider"
import {styled} from "@mui/material/styles"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"

const Hidden = styled(Box)`
    display: none;
`

interface IElectionSubmit {
    description: string
    name: string
}

interface IElectionEventSubmit {
    name: string
    description: string
    elections: Array<IElectionSubmit>
    encryption_protocol: string
    id: string
    tenant_id: string
}

export const CreateElectionList: React.FC = () => {
    const [insertElectionEvent] = useMutation<CreateElectionEventMutation>(INSERT_ELECTION_EVENT)
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const {t} = useTranslation()
    const navigate = useNavigate()
    const refresh = useRefresh()
    const postDefaultValues = () => ({id: v4()})
    const {
        data: newElectionEvent,
        isLoading: isOneLoading,
        error,
    } = useGetOne("sequent_backend_election_event", {
        id: newId,
    })

    const {refetch: refetchTreeMenu} = useTreeMenuData(false)

    useEffect(() => {
        if (isNull(newId)) {
            return
        }
        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            notify(t("electionEventScreen.createElectionEventError"), {type: "error"})
            refresh()
            return
        }
        if (isLoading && !error && !isOneLoading && newElectionEvent) {
            setIsLoading(false)
            notify(t("electionEventScreen.createElectionEventSuccess"), {type: "success"})
            refresh()
            navigate(`/sequent_backend_election_event/${newId}`)
        }
    }, [isLoading, newElectionEvent, isOneLoading, error])

    const handleSubmit = async (values: any) => {
        const electionSubmit = values as IElectionEventSubmit
        let {data, errors} = await insertElectionEvent({
            variables: {
                electionEvent: electionSubmit,
            },
        })

        if (data?.insertElectionEvent?.id) {
            setNewId(data?.insertElectionEvent?.id)
            setIsLoading(true)
        } else {
            notify(t("electionEventScreen.createElectionEventError"), {type: "error"})
            setIsLoading(false)
        }
        refresh()
        refetchTreeMenu()
    }
    return (
        <SimpleForm defaultValues={postDefaultValues} onSubmit={handleSubmit}>
            <Typography variant="h4">{t("electionEventScreen.common.title")}</Typography>
            <Typography variant="body2">{t("electionEventScreen.new.subtitle")}</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <Hidden>
                <SelectInput
                    source="encryption_protocol"
                    choices={[{id: "RSA256", name: "RSA256"}]}
                    defaultValue={"RSA256"}
                />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="slug" defaultValue={tenantId} />
                </ReferenceInput>
                <BooleanInput source="is_archived" defaultValue={false} />
                <JsonInput
                    source="labels"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="presentation"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="voting_channels"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="voting_channels"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="dates"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <TextInput source="user_boards" />
                <TextInput source="audit_election_event_id" />
            </Hidden>
            {isLoading ? <CircularProgress /> : null}
        </SimpleForm>
    )
}
