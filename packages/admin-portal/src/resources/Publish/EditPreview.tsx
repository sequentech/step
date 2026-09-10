// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
import {
    AutocompleteInput,
    Button,
    Identifier,
    SaveButton,
    SimpleForm,
    Toolbar,
    useNotify,
} from "react-admin"
import {Preview, ContentCopy} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {PrepareBallotPublicationPreviewMutation, GetTaskByIdQuery} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {gql, useApolloClient, useMutation, useQuery} from "@apollo/client"
import {PREPARE_BALLOT_PUBLICATION_PREVIEW} from "@/queries/PrepareBallotPublicationPreview"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {TenantContext} from "@/providers/TenantContextProvider"
import {CircularProgress} from "@mui/material"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"

enum ActionType {
    Copy,
    Open,
}
interface EditPreviewProps {
    publicationId?: string | Identifier | null
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
    const {publicationId, close, electionEventId} = props
    const {t} = useTranslation()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [sourceAreas, setSourceAreas] = useState<Array<{id: string; name: string}>>([])
    const client = useApolloClient()
    const [taskId, setTaskId] = useState<string | null>(null)
    const [preparePreview] = useMutation<PrepareBallotPublicationPreviewMutation>(
        PREPARE_BALLOT_PUBLICATION_PREVIEW
    )
    const [isUploading, setIsUploading] = React.useState<boolean>(false)
    const {tenantId} = useContext(TenantContext)
    const [areaId, setAreaId] = useState<string | null>(null)
    const [documentId, setDocumentId] = useState<string | null | undefined>(null)
    const [action, setAction] = useState<ActionType | null>(null)
    const {data: taskData} = useQuery<GetTaskByIdQuery>(GET_TASK_BY_ID, {
        variables: {task_id: taskId},
        skip: !taskId,
        pollInterval: taskId ? globalSettings.QUERY_POLL_INTERVAL_MS : 0,
    })

    // Load identifiers, not the truncated and potentially very large EML diff.
    useEffect(() => {
        let active = true
        setSourceAreas([])
        const loadAreas = async () => {
            const result: Array<{id: string; name: string}> = []
            for (let offset = 0; ; offset += 1000) {
                const {data} = await client.query({
                    query: gql`
                        query PublicationPreviewAreas(
                            $publicationId: uuid!
                            $eventId: uuid!
                            $offset: Int!
                        ) {
                            sequent_backend_ballot_style(
                                where: {
                                    ballot_publication_id: {_eq: $publicationId}
                                    election_event_id: {_eq: $eventId}
                                }
                                distinct_on: area_id
                                order_by: {area_id: asc}
                                limit: 1000
                                offset: $offset
                            ) {
                                area_id
                            }
                        }
                    `,
                    variables: {publicationId, eventId: electionEventId, offset},
                    fetchPolicy: "network-only",
                })
                if (!active) return
                const ids = data.sequent_backend_ballot_style.map(
                    (style: {area_id: string}) => style.area_id
                )
                if (ids.length) {
                    const {data: areas} = await client.query({
                        query: gql`
                            query PublicationPreviewAreaNames($ids: [uuid!]!, $eventId: uuid!) {
                                sequent_backend_area(
                                    where: {id: {_in: $ids}, election_event_id: {_eq: $eventId}}
                                ) {
                                    id
                                    name
                                }
                            }
                        `,
                        variables: {ids, eventId: electionEventId},
                        fetchPolicy: "network-only",
                    })
                    result.push(...areas.sequent_backend_area)
                }
                if (ids.length < 1000) break
            }
            if (active) setSourceAreas(result)
        }
        if (publicationId && electionEventId) {
            loadAreas().catch(() => {
                if (active) notify(t("publish.dialog.error_preview"), {type: "error"})
            })
        }
        return () => {
            active = false
        }
    }, [client, publicationId, electionEventId, notify, t])

    const task = taskData?.sequent_backend_tasks_execution?.[0]
    useEffect(() => {
        if (!taskId || task?.id !== taskId) return
        if (task.execution_status === ETaskExecutionStatus.SUCCESS) {
            setTaskId(null)
            setIsUploading(false)
        } else if (task.execution_status === ETaskExecutionStatus.FAILED) {
            setTaskId(null)
            setDocumentId(null)
            setAction(null)
            setIsUploading(false)
            notify(t("publish.dialog.error_preview"), {type: "error"})
        }
    }, [taskId, task, notify, t])

    useEffect(() => {
        let active = true
        if (!isUploading || taskId || documentId) return
        const widget = addWidget(ETasksExecution.PREPARE_PUBLICATION_PREVIEW, undefined)
        preparePreview({variables: {electionEventId, ballotPublicationId: publicationId}})
            .then(({data}) => {
                const output = data?.prepare_ballot_publication_preview
                if (output?.error_msg || !output?.document_id || !output.task_execution?.id) {
                    throw new Error(output?.error_msg || "Preview task was not created")
                }
                setWidgetTaskId(widget.identifier, output.task_execution.id)
                if (active) {
                    setDocumentId(output.document_id)
                    setTaskId(output.task_execution.id)
                }
            })
            .catch(() => {
                updateWidgetFail(widget.identifier)
                if (active) {
                    setDocumentId(null)
                    setAction(null)
                    setIsUploading(false)
                    notify(t("publish.dialog.error_preview"), {type: "error"})
                }
            })
        return () => {
            active = false
        }
    }, [isUploading])

    const onPreviewClick = async (res: any) => {
        if (!documentId) {
            setIsUploading(true)
        }
        setAction(ActionType.Open)
    }

    const onCopyPreviewLinkClick = async () => {
        if (!documentId) {
            setIsUploading(true)
        }
        setAction(ActionType.Copy)
    }

    // This useEffect handles logic for action (open or copy)
    useEffect(() => {
        const openPreview = (previewUrl: string) => {
            try {
                window.open(previewUrl, "_blank")
                if (close) close()
                notify(t("publish.preview.success"), {type: "success"})
            } catch {
                notify(t("publish.dialog.error_preview"), {type: "error"})
            }
        }

        const copyPreviewLink = async (previewUrl: string) => {
            try {
                await navigator.clipboard.writeText(previewUrl)
                if (close) close()
                notify(t("publish.preview.copy_success"), {type: "success"})
            } catch {
                notify(t("publish.preview.copy_error"), {type: "error"})
            }
        }

        if (documentId && !isUploading && !taskId) {
            const previewUrl = getPreviewUrl(documentId)
            if (previewUrl && action === ActionType.Copy) {
                copyPreviewLink(previewUrl)
            } else if (previewUrl && action === ActionType.Open) {
                openPreview(previewUrl)
            }
        }
    }, [documentId, action, isUploading, taskId])

    // Create preview url from data
    const previewUrlTemplate = useMemo(() => {
        return `${globalSettings.VOTING_PORTAL_URL}/preview/${tenantId}`
    }, [globalSettings.VOTING_PORTAL_URL, tenantId])

    const getPreviewUrl = useCallback(
        (documentId: string | undefined | null) => {
            if (!documentId || !areaId || !publicationId) {
                return null
            }
            return `${previewUrlTemplate}/${documentId}/${areaId}/${publicationId}`
        },
        [previewUrlTemplate, areaId, publicationId]
    )

    return (
        <SimpleForm toolbar={false} onSubmit={onPreviewClick}>
            <AutocompleteInput
                source="area_id"
                choices={sourceAreas}
                optionText={(area) => area.name}
                label={String(t("publish.preview.publicationAreas"))}
                fullWidth={true}
                debounce={100}
                onChange={(res) => setAreaId(res)}
            ></AutocompleteInput>
            <Toolbar
                sx={{display: "flex", background: "white", padding: "0 !important", gap: "1rem"}}
            >
                {isUploading ? (
                    <CircularProgress />
                ) : (
                    <>
                        <SaveButton
                            disabled={!areaId}
                            icon={<Preview />}
                            label={String(t("publish.preview.action"))}
                        />
                        <Button
                            disabled={!areaId}
                            startIcon={<ContentCopy />}
                            label={String(t("publish.preview.copy"))}
                            onClick={onCopyPreviewLinkClick}
                        />
                    </>
                )}
            </Toolbar>
        </SimpleForm>
    )
}
