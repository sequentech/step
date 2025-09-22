// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMutation, useQuery} from "@apollo/client"
import React, {useContext, useEffect, useState} from "react"
import {
    CreateElectionEventMutation,
    ImportElectionEventMutation,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {v4} from "uuid"
import {
    BooleanInput,
    ReferenceInput,
    SaveButton,
    Toolbar,
    SelectInput,
    SimpleForm,
    TextInput,
    useGetOne,
    useNotify,
    useRefresh,
    RaRecord,
    useGetList,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {INSERT_ELECTION_EVENT} from "../../queries/InsertElectionEvent"
import {Box, CircularProgress, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {IElectionEventPresentation, ITenantSettings, isNull} from "@sequentech/ui-core"
import {useNavigate} from "react-router"
import {useTenantStore} from "../../providers/TenantContextProvider"
import UploadIcon from "@mui/icons-material/Upload"
import {styled} from "@mui/material/styles"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {IMPORT_ELECTION_EVENT} from "@/queries/ImportElectionEvent"
import {ExportButton} from "@/components/tally/ExportElectionMenu"
import {addDefaultTranslationsToElement} from "@/services/i18n"
import {ETasksExecution} from "@/types/tasksExecution"
import {useActionPermissions} from "../../components/menu/items/use-tree-menu-hook"

const Hidden = styled(Box)`
    display: none;
`

const ReservedSpace = styled(Box)`
    min-height: 40px;
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
    presentation: IElectionEventPresentation
}
interface IPullChecker<T extends RaRecord> {
    id: string
    resource: string
    dependencies: any[]
    onResolved: (result: {data: T[] | undefined; isLoading: boolean; error: any}) => void
}

const PullChecker = <T extends RaRecord>({
    id,
    resource,
    dependencies,
    onResolved,
}: IPullChecker<T>) => {
    const {globalSettings} = useContext(SettingsContext)

    const {data, isLoading, error} = useGetList<T>(
        resource,
        {filter: {id: id}},
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    useEffect(() => {
        onResolved({data, isLoading, error})
    }, [isLoading, data, error, id, ...dependencies])

    return <div />
}

export const CreateElectionList: React.FC = () => {
    const [insertElectionEvent] = useMutation<CreateElectionEventMutation>(INSERT_ELECTION_EVENT)
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const [settings, setSettings] = useState<any>()
    const {t} = useTranslation()
    const navigate = useNavigate()
    const refresh = useRefresh()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const {setLastCreatedResource} = useContext(NewResourceContext)
    const {refetch: refetchTreeMenu} = useTreeMenuData(false)

    const postDefaultValues = () => ({id: v4()})

    const {data: tenant} = useGetOne("sequent_backend_tenant", {
        id: tenantId,
    })

    useEffect(() => {
        if (tenant) {
            const temp = tenant?.settings
            setSettings(temp)
        }
    }, [tenant])

    const handleElectionCreated = ({
        error,
        isLoading: isOneLoading,
        data: newElectionEvent,
    }: {
        data: Sequent_Backend_Election_Event[] | undefined
        isLoading: boolean
        error: any
    }) => {
        if (isNull(newId)) {
            return
        }

        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            notify(t("electionEventScreen.createElectionEventError"), {type: "error"})
            refresh()
            return
        }
        if (isLoading && !error && !isOneLoading && newElectionEvent!.length) {
            setIsLoading(false)
            notify(t("electionEventScreen.createElectionEventSuccess"), {type: "success"})
            setLastCreatedResource({id: newId, type: "sequent_backend_election_event"})
            refresh()
        }
    }

    const handleSubmit = async (values: any): Promise<void> => {
        let electionSubmit = values as IElectionEventSubmit
        let i18n = addDefaultTranslationsToElement(electionSubmit)
        let tenantLangConf = (tenant?.settings as ITenantSettings | undefined)?.language_conf ?? {
            enabled_language_codes: settings?.languages ?? ["en"],
            default_language_code: "en",
        }
        tenantLangConf.default_language_code = tenantLangConf.default_language_code ?? "en"

        let presentation: IElectionEventPresentation = {
            ...(values.presentation as IElectionEventPresentation),
            i18n,
            language_conf: tenantLangConf,
        }

        electionSubmit = {
            ...electionSubmit,
            presentation,
        }

        try {
            let {data, errors} = await insertElectionEvent({
                variables: {
                    electionEvent: electionSubmit,
                },
            })

            const newId = data?.insertElectionEvent?.id ?? null
            if (newId) {
                setNewId(newId)
                setLastCreatedResource({id: newId, type: "sequent_backend_election_event"})
                setIsLoading(true)
            } else {
                console.log(`Error creating Election Event ${errors}`)
                notify(t("electionEventScreen.createElectionEventError"), {type: "error"})
                setIsLoading(false)
            }
        } catch (error) {
            console.log(`Error creating Election Event ${error}`)
            notify(t("electionEventScreen.createElectionEventError"), {type: "error"})
            setIsLoading(false)
        }

        refresh()

        setTimeout(() => {
            refetchTreeMenu()
        }, globalSettings.QUERY_POLL_INTERVAL_MS)
    }

    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [errors, setErrors] = useState<string | null>(null)

    const closeImportDrawer = () => {
        setOpenDrawer(false)
        setErrors(null)
    }

    /**
     * permissions
     */
    const {canWriteElectionEvent} = useActionPermissions()

    return (
        <>
            {newId && (
                <PullChecker<Sequent_Backend_Election_Event>
                    id={newId}
                    resource="sequent_backend_election_event"
                    dependencies={[isLoading, newId]}
                    onResolved={handleElectionCreated}
                />
            )}
            <SimpleForm
                defaultValues={postDefaultValues}
                onSubmit={handleSubmit}
                toolbar={
                    <Toolbar>
                        {canWriteElectionEvent && (
                            <SaveButton
                                className="election-event-save-button"
                                disabled={isLoading}
                            />
                        )}
                    </Toolbar>
                }
            >
                <Box
                    sx={{
                        display: "flex",
                        flexDirection: "row",
                        justifyContent: "space-between",
                        alignItems: "center",
                        width: "100%",
                    }}
                >
                    <Typography variant="h4">{t("common.resources.electionEvent")}</Typography>
                    <ExportButton onClick={() => setOpenDrawer(true)}>
                        <UploadIcon />
                        {t("common.label.import")}
                    </ExportButton>
                </Box>
                <Typography variant="body2">{t("createResource.electionEvent")}</Typography>
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
                <ReservedSpace>{isLoading ? <CircularProgress /> : null}</ReservedSpace>
            </SimpleForm>
        </>
    )
}
