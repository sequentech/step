// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {FormStyles} from "@/components/styles/FormStyles"
import {WidgetProps} from "@/components/Widget"
import {ExportBallotPublicationMutation, Sequent_Backend_Election} from "@/gql/graphql"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {EXPORT_BALLOT_PUBLICATION} from "@/queries/ExportBallotPublication"
import {IPermissions} from "@/types/keycloak"
import {ETasksExecution} from "@/types/tasksExecution"
import {useMutation} from "@apollo/client"
import {Dialog} from "@sequentech/ui-essentials"
import React, {FC, useState} from "react"
import {useTranslation} from "react-i18next"
import {DownloadDocument} from "../User/DownloadDocument"
import {Identifier, useRecordContext} from "react-admin"
import {ListActions} from "@/components/ListActions"
import {useTenantStore} from "@/providers/TenantContextProvider"

interface PublishExportProps {
    ballotPublicationId?: Identifier | null
}

const PublishExport: FC<PublishExportProps> = ({ballotPublicationId}) => {
    const [openExport, setOpenExport] = useState(false)
    const [exporting, setExporting] = useState(false)
    const record = useRecordContext<Sequent_Backend_Election>()
    const [tenantId] = useTenantStore()
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const {t} = useTranslation()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [ExportBallotPublication] = useMutation<ExportBallotPublicationMutation>(
        EXPORT_BALLOT_PUBLICATION,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.PUBLISH_WRITE,
                },
            },
        }
    )

    const handleExport = async () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        let currWidget: WidgetProps | undefined
        try {
            currWidget = addWidget(ETasksExecution.EXPORT_BALLOT_PUBLICATION, undefined)

            const {data: ballotResponse, errors} = await ExportBallotPublication({
                variables: {
                    tenantId,
                    electionEventId: record?.election_event_id
                        ? record?.election_event_id
                        : record?.id,
                    electionId: record?.election_event_id ? record?.id : null,
                    ballotPublicationId: ballotPublicationId,
                },
            })

            setExporting(true)
            if (errors) {
                setExporting(false)
                updateWidgetFail(currWidget.identifier)

                return
            }
            const documentId = ballotResponse?.export_ballot_publication?.document_id
            setExportDocumentId(documentId)
            const task_id = ballotResponse?.export_ballot_publication?.task_execution?.id
            task_id
                ? setWidgetTaskId(currWidget.identifier, task_id)
                : updateWidgetFail(currWidget.identifier)
        } catch (error) {
            console.log(error)
            setExporting(false)
            currWidget && updateWidgetFail(currWidget.identifier)
        }
    }

    return (
        <>
            <ListActions
                withExport={true}
                withColumns={false}
                withImport={false}
                withFilter={false}
                doExport={handleExport}
            />
            <Dialog
                variant="info"
                open={openExport}
                ok={String(t("common.label.export"))}
                okEnabled={() => !exporting}
                cancel={String(t("common.label.cancel"))}
                title={String(t("common.label.export"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setExportDocumentId(undefined)
                        setExporting(false)
                        setOpenExport(false)
                    }
                }}
            >
                {t("common.export")}
                <FormStyles.ReservedProgressSpace>
                    {exporting ? <FormStyles.ShowProgress /> : null}
                    {exporting && exportDocumentId ? (
                        <DownloadDocument
                            documentId={exportDocumentId}
                            fileName={`ballot-publication-export.json`}
                            onDownload={() => {
                                setExportDocumentId(undefined)
                                setExporting(false)
                                setOpenExport(false)
                            }}
                        />
                    ) : null}
                </FormStyles.ReservedProgressSpace>
            </Dialog>
        </>
    )
}

export default PublishExport
