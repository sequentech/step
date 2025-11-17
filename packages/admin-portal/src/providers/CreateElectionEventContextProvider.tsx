// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {
    createContext,
    Dispatch,
    SetStateAction,
    useContext,
    useEffect,
    useState,
} from "react"

import {useMutation} from "@apollo/client"
import {
    CreateElectionEventMutation,
    ImportElectionEventMutation,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {v4} from "uuid"
import {useGetOne, useNotify, useRefresh, RaRecord, useGetList} from "react-admin"
import {useTranslation} from "react-i18next"
import {IElectionEventPresentation, ITenantSettings, isNull} from "@sequentech/ui-core"
import {useNavigate} from "react-router"
import {useTreeMenuData} from "@/components/menu/items/use-tree-menu-hook"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {IMPORT_ELECTION_EVENT} from "@/queries/ImportElectionEvent"
import {addDefaultTranslationsToElement} from "@/services/i18n"
import {ETasksExecution} from "@/types/tasksExecution"
import {INSERT_ELECTION_EVENT} from "@/queries/InsertElectionEvent"
import {useTenantStore} from "./TenantContextProvider"
import {useAtom} from "jotai"
import archivedElectionEventSelection from "@/atoms/archived-election-event-selection"

interface IElectionSubmit {
    description: string
    name: string
}

export interface IElectionEventSubmit {
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

const CreateElectionEventContext = createContext<{
    createDrawer: boolean
    importDrawer: boolean
    openCreateDrawer: () => void
    closeCreateDrawer?: () => void
    openImportDrawer: () => void
    closeImportDrawer: () => void
    postDefaultValues: any
    handleElectionCreated: any
    uploadCallback: any
    handleImportElectionEvent: any
    handleSubmit: any
    errors: any
    isLoading: boolean
    newId: any
    tenantId: any
}>({
    createDrawer: false,
    importDrawer: false,
    postDefaultValues: console.log,
    handleElectionCreated: console.log,
    uploadCallback: console.log,
    handleImportElectionEvent: console.log,
    handleSubmit: console.log,
    openCreateDrawer: console.log,
    closeCreateDrawer: console.log,
    openImportDrawer: console.log,
    closeImportDrawer: console.log,
    errors: null,
    isLoading: false,
    newId: false,
    tenantId: "",
})

export const CreateElectionEventProvider = ({children}: any) => {
    const [createDrawer, toggleCreateDrawer] = useState(false)
    const [importDrawer, toggleImportDrawer] = useState(false)

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

    const [isArchivedElectionEvents, setArchivedElectionEvents] = useAtom(
        archivedElectionEventSelection
    )

    const postDefaultValues = () => ({id: v4()})

    const {data: tenant} = useGetOne("sequent_backend_tenant", {
        id: tenantId,
    })

    const openCreateDrawer = () => {
        setIsLoading(false)
        toggleCreateDrawer(true)
    }

    const closeCreateDrawer = () => {
        toggleCreateDrawer(false)
    }

    const openImportDrawer = () => {
        setErrors(null)
        toggleImportDrawer(true)
    }

    const closeImportDrawer = () => {
        toggleImportDrawer(false)
    }

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
            setIsLoading(false)
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
            refresh()
            navigate(`/sequent_backend_election_event/${newId}`)
        }
    }

    const handleSubmit = async (values: any): Promise<void> => {
        const currWidget = addWidget(ETasksExecution.CREATE_ELECTION_EVENT, undefined)
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

        closeCreateDrawer()

        try {
            let {data, errors} = await insertElectionEvent({
                variables: {
                    electionEvent: electionSubmit,
                },
            })

            const newId = data?.insertElectionEvent?.id ?? null
            if (newId) {
                setNewId(newId)
                setWidgetTaskId(
                    currWidget.identifier,
                    data?.insertElectionEvent?.task_execution?.id,
                    () => navigate(`/sequent_backend_election_event/${newId}`)
                )
                setLastCreatedResource({id: newId, type: "sequent_backend_election_event"})
                setIsLoading(true)
            } else {
                updateWidgetFail(currWidget.identifier)
                setIsLoading(false)
            }
        } catch (error) {
            setIsLoading(false)
            updateWidgetFail(currWidget.identifier)
        }
    }

    const [errors, setErrors] = useState<string | null>(null)
    const [importElectionEvent] = useMutation<ImportElectionEventMutation>(IMPORT_ELECTION_EVENT)

    // const closeImportDrawer = () => {
    //     toggleImportDrawer((prev) => !prev)
    //     setErrors(null)
    // }

    const uploadCallback = async (documentId: string, password: string = "") => {
        setErrors(null)
        let {data: importData, errors} = await importElectionEvent({
            variables: {
                tenantId,
                documentId,
                password,
                checkOnly: true,
            },
        })

        if (importData?.import_election_event?.error) {
            setErrors(importData.import_election_event.error)
            throw new Error(importData?.import_election_event?.error)
        }
    }

    const handleImportElectionEvent = async (
        documentId: string,
        sha256: string,
        password?: string
    ) => {
        closeImportDrawer()
        setIsLoading(false)
        setErrors(null)

        const currWidget = addWidget(ETasksExecution.IMPORT_ELECTION_EVENT, undefined)

        try {
            let {data, errors} = await importElectionEvent({
                variables: {
                    tenantId,
                    documentId,
                    password,
                },
            })
            if (data?.import_election_event?.error) {
                setErrors(data.import_election_event.error)
                updateWidgetFail(currWidget.identifier)
                return
            }

            let id = data?.import_election_event?.id
            if (id) {
                setWidgetTaskId(
                    currWidget.identifier,
                    data?.import_election_event?.task_execution?.id,
                    () => navigate(`/sequent_backend_election_event/${id}`)
                )
                setNewId(id)
                setLastCreatedResource({id, type: "sequent_backend_election_event"})
                setArchivedElectionEvents(false)
            }
        } catch (err) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    return (
        <CreateElectionEventContext.Provider
            value={{
                createDrawer,
                importDrawer,
                openCreateDrawer,
                closeCreateDrawer,
                openImportDrawer,
                closeImportDrawer,
                postDefaultValues,
                handleElectionCreated,
                uploadCallback,
                handleImportElectionEvent,
                handleSubmit,
                errors,
                isLoading,
                newId,
                tenantId,
            }}
        >
            {children}
        </CreateElectionEventContext.Provider>
    )
}

//hook
export const useCreateElectionEventStore = () => useContext(CreateElectionEventContext)
