// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMutation} from "@apollo/client"
import React, {useContext, useEffect, useState} from "react"
import {CreateElectionEventMutation, ImportElectionEventMutation} from "@/gql/graphql"
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
    Button,
    RaRecord,
    Identifier,
    useGetList,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {INSERT_ELECTION_EVENT} from "../../queries/InsertElectionEvent"
import {Box, CircularProgress, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {IElectionEventPresentation, ITenantSettings, isNull} from "@sequentech/ui-essentials"
import {useNavigate} from "react-router"
import {useTenantStore} from "../../providers/TenantContextProvider"
import UploadIcon from "@mui/icons-material/Upload"
import {styled} from "@mui/material/styles"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {IMPORT_ELECTION_EVENT} from "@/queries/ImportElectionEvent"
import {ExportButton} from "@/components/tally/ExportElectionMenu"
import {Sequent_Backend_Election_Event_Extended} from "./EditElectionEventDataForm"
import {addDefaultTranslationsToElement} from "@/services/i18n"

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

    const postDefaultValues = () => ({id: v4()})

    // const {
    //     data: newElectionEvent,
    //     isLoading: isOneLoading,
    //     error,
    // } = useGetOne(
    //     "sequent_backend_election_event",
    //     {id: newId},
    //     {
    //         refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    //     }
    // )

    const {
        data: newElectionEvent,
        isLoading: isOneLoading,
        error,
    } = useGetList("sequent_backend_election_event", {filter: {id: newId}})

    console.log("electionEventScreenLogs", {
        error,
        isLoading,
        isOneLoading,
        newElectionEvent,
        newId,
    })
    const {data: tenant} = useGetOne("sequent_backend_tenant", {
        id: tenantId,
    })

    const {setLastCreatedResource} = useContext(NewResourceContext)
    const {refetch: refetchTreeMenu} = useTreeMenuData(false)

    useEffect(() => {
        if (tenant) {
            const temp = tenant?.settings
            setSettings(temp)
        }
    }, [tenant])

    useEffect(() => {
        if (isNull(newId)) {
            return
        }
        console.log("effect", {error, isLoading, isOneLoading})

        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            console.warn("error 3")
            notify(t("electionEventScreen.createElectionEventError"), {type: "error"})
            refresh()
            return
        }
        if (
            isLoading &&
            !error &&
            !isOneLoading &&
            (newElectionEvent as Sequent_Backend_Election_Event_Extended[]).length
        ) {
            console.warn("success")

            setIsLoading(false)
            notify(t("electionEventScreen.createElectionEventSuccess"), {type: "success"})
            refresh()
            // navigate(`/sequent_backend_election_event/${newId}`)
        }
    }, [isLoading, newElectionEvent, isOneLoading, error])

    const handleSubmit = async (values: any): Promise<void> => {
        console.warn("error 1")
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

        console.log("electionSubmit :: ", electionSubmit)

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
                console.warn("error 1")
                notify("electionEventScreen.createElectionEventError 2", {type: "info"})
                setIsLoading(false)
            }
        } catch (error) {
            console.log(`Error creating Election Event ${error}`)
            console.warn("error 2")
            notify("electionEventScreen.createElectionEventError 3", {type: "success"})
            setIsLoading(false)
        }

        refresh()

        setTimeout(() => {
            refetchTreeMenu()
        }, globalSettings.QUERY_POLL_INTERVAL_MS)
    }

    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [errors, setErrors] = useState<string | null>(null)
    const [importElectionEvent] = useMutation<ImportElectionEventMutation>(IMPORT_ELECTION_EVENT)

    const closeImportDrawer = () => {
        setOpenDrawer(false)
        setErrors(null)
    }

    const uploadCallback = async (documentId: string) => {
        setErrors(null)
        let {data, errors} = await importElectionEvent({
            variables: {
                tenantId,
                documentId,
                checkOnly: true,
            },
        })

        if (data?.import_election_event?.error) {
            setErrors(data.import_election_event.error)
            throw new Error(data?.import_election_event?.error)
        }
    }

    const handleImportElectionEvent = async (documentId: string, sha256: string) => {
        setErrors(null)
        let {data, errors} = await importElectionEvent({
            variables: {
                tenantId,
                documentId,
            },
        })
        console.log("election event imported", {data, errors})

        if (data?.import_election_event?.error) {
            console.log("election event imported err", {data, errors})

            setErrors(data.import_election_event.error)
            return
        }

        let id = data?.import_election_event?.id
        if (id) {
            console.log("election event imported set id", {data, errors})

            setNewId(id)
            setLastCreatedResource({id, type: "sequent_backend_election_event"})
            setIsLoading(true)
        }
    }

    return (
        <>
            <SimpleForm
                defaultValues={postDefaultValues}
                onSubmit={handleSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton className="election-event-save-button" disabled={isLoading} />
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

            <hr />

            <ImportDataDrawer
                open={openDrawer}
                closeDrawer={closeImportDrawer}
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph={"electionEventScreen.import.electionEventParagraph"}
                doImport={handleImportElectionEvent}
                uploadCallback={uploadCallback}
                errors={errors}
            />
        </>
    )
}
