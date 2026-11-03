// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, CircularProgress, Menu, MenuItem} from "@mui/material"
import React, {useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useNotify} from "react-admin"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"
import {Dialog} from "@sequentech/ui-essentials"
import {sanitizeFilename, FOLDER_MAX_CHARS, downloadBlob} from "@sequentech/ui-core"
import {IMiruDocument} from "@/types/miru"
import {TallyStyles} from "@/components/styles/TallyStyles"
import DownloadIcon from "@mui/icons-material/Download"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {GENERATE_TRANSMISSION_REPORT} from "@/queries/GenerateTransmissionReport"
import {RECORD_TRANSMISSION_DOWNLOAD} from "@/queries/RecordTransmissionDownload"
import {RecordTransmissionDownloadMutation} from "@/gql/graphql"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {
    ETransmissionFile,
    latestDocument,
    transmissionFileName,
} from "@/services/transmissionPackage"

interface MiruPackageDownloadProps {
    documents: IMiruDocument[] | null
    areaName: string | null | undefined
    tenantId: string
    electionEventId: string
    electionId?: string
    areaId?: string
    tallySessionId?: string
    eventName: string
    /** The OVCS public key PEM the package's signature verifies with. */
    ovcsPublicKey?: string | null
}

interface IPendingDownload {
    documentId: string
    fileName: string
}

const PEM_MEDIA_TYPE = "application/x-pem-file"

const formatDate = (date: Date): string => {
    const options: Intl.DateTimeFormatOptions = {
        day: "2-digit",
        month: "2-digit",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false, // Use 24-hour format
    }

    const formatter = new Intl.DateTimeFormat(undefined, options)
    const parts = formatter.formatToParts(date)

    // Extract parts to create the desired format
    const day = parts.find((part) => part.type === "day")?.value
    const month = parts.find((part) => part.type === "month")?.value
    const year = parts.find((part) => part.type === "year")?.value
    const hour = parts.find((part) => part.type === "hour")?.value
    const minute = parts.find((part) => part.type === "minute")?.value

    return `${day}/${month}/${year} ${hour}:${minute}`
}

const menuItemText = {
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    overflow: "hidden",
}

export const MiruPackageDownload: React.FC<MiruPackageDownloadProps> = ({
    areaName,
    documents,
    tenantId,
    electionEventId,
    electionId,
    areaId,
    tallySessionId,
    eventName,
    ovcsPublicKey,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const zoned = useEventZonedFormat(electionEventId)
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [pending, setPending] = useState<IPendingDownload | null>(null)
    const [recording, setRecording] = useState<boolean>(false)
    const [documentToDownload, setDocumentToDownload] = useState<string | null>(null)
    const [fileNameWithExt, setFileNameWithExt] = useState<string>("all_servers.tar.gz")
    const [performDownload, setPerformDownload] = useState<boolean>(false)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    const [generatTransmissionReport] = useMutation(GENERATE_TRANSMISSION_REPORT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.TRANSMISSION_REPORT_GENERATE,
            },
        },
    })

    const [recordTransmissionDownload] = useMutation<RecordTransmissionDownloadMutation>(
        RECORD_TRANSMISSION_DOWNLOAD,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.MIRU_DOWNLOAD,
                },
            },
        }
    )

    const fileName = useMemo(() => {
        const sanitizedAreaName = sanitizeFilename(
            `area__${areaName ?? ""}`,
            Math.floor(FOLDER_MAX_CHARS / 2)
        )
        const sanitizedEventName = sanitizeFilename(
            `__event__${eventName}`,
            Math.floor(FOLDER_MAX_CHARS / 2)
        )
        return sanitizeFilename(`${sanitizedAreaName}-${sanitizedEventName}`)
    }, [areaName, eventName])

    const handleMenu = (event: React.MouseEvent<HTMLElement>) => {
        event.preventDefault()
        event.stopPropagation()
        setAnchorEl(event.currentTarget)
    }

    const handleClose = () => {
        setAnchorEl(null)
    }

    const onDownloadReport = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        handleClose()
        const currWidget = addWidget(ETasksExecution.GENERATE_TRANSMISSION_REPORT, undefined)
        try {
            let generateReportResponse = await generatTransmissionReport({
                variables: {
                    tenantId,
                    electionEventId,
                    electionId,
                    tallySessionId,
                },
            })
            let taskId =
                generateReportResponse?.data?.generate_transmission_report.task_execution?.id
            let generatedDocumentId =
                generateReportResponse?.data?.generate_transmission_report?.document_id

            if (!generatedDocumentId) {
                updateWidgetFail(currWidget.identifier)
                return
            }
            setFileNameWithExt("transmission_report" + ".pdf")
            setDocumentToDownload(generatedDocumentId)
            setPerformDownload(true)
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    const lastDocument = useMemo(() => latestDocument(documents), [documents])

    // The file name keeps its compact date; the menu shows the time with its zone label.
    const lastDocumentDate =
        (lastDocument?.created_at && formatDate(new Date(lastDocument?.created_at))) || ""
    const lastDocumentTime = zoned.format(lastDocument?.created_at)

    const emlDocumentId = lastDocument?.document_ids.eml
    const csvDocumentId = lastDocument?.document_ids.csv
    const packageDocumentId = lastDocument?.document_ids.all_servers

    const askToDownload = (documentId: string, file: ETransmissionFile) => {
        handleClose()
        setPending({
            documentId,
            fileName: transmissionFileName(fileName, lastDocumentDate, file),
        })
    }

    const onMenuItem =
        (documentId: string, file: ETransmissionFile) => (e: React.MouseEvent<HTMLElement>) => {
            e.preventDefault()
            e.stopPropagation()
            askToDownload(documentId, file)
        }

    /** Every download of a package file is logged first; without the record there is no download. */
    const recordDownload = async (documentId: string): Promise<boolean> => {
        const failed = () => {
            notify(t("tally.transmissionPackage.actions.download.recordError"), {type: "error"})
            return false
        }
        if (!electionId || !areaId || !tallySessionId) {
            return failed()
        }
        try {
            const {data, errors} = await recordTransmissionDownload({
                variables: {
                    electionEventId,
                    electionId,
                    areaId,
                    tallySessionId,
                    documentId,
                },
            })
            if (errors?.length || !data?.record_transmission_download) {
                return failed()
            }
            return true
        } catch (error) {
            console.log(`Caught error: ${error}`)
            return failed()
        }
    }

    const onConfirm = async (result: boolean) => {
        const download = pending
        setPending(null)
        if (!result || !download) {
            return
        }
        setRecording(true)
        try {
            if (await recordDownload(download.documentId)) {
                setFileNameWithExt(download.fileName)
                setDocumentToDownload(download.documentId)
                setPerformDownload(true)
            }
        } finally {
            setRecording(false)
        }
    }

    const onDownloadOvcsKey = async (e: React.MouseEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        handleClose()
        if (!ovcsPublicKey) {
            return
        }
        try {
            await downloadBlob(
                new Blob([ovcsPublicKey], {type: PEM_MEDIA_TYPE}),
                transmissionFileName(fileName, "", ETransmissionFile.OVCS_PUBLIC_KEY)
            )
        } catch (error) {
            console.log(`Caught error: ${error}`)
        }
    }

    const item = (title: string, label: string = title) => (
        <Box sx={menuItemText}>
            <span title={title}>{label}</span>
        </Box>
    )

    return (
        <Box>
            <TallyStyles.MiruToolbarButton
                variant="outlined"
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                {recording ? <CircularProgress size={16} /> : <DownloadIcon />}
                <span title={String(t("tally.transmissionPackage.actions.download.title"))}>
                    {t("tally.transmissionPackage.actions.download.title")}
                </span>
                {performDownload && documentToDownload ? (
                    <DownloadDocument
                        onDownload={() => {
                            setDocumentToDownload(null)
                            setPerformDownload(false)
                        }}
                        fileName={fileNameWithExt}
                        documentId={documentToDownload}
                        electionEventId={electionEventId}
                        withProgress={true}
                    />
                ) : null}
            </TallyStyles.MiruToolbarButton>

            <Menu
                id="menu-export-election"
                anchorEl={anchorEl}
                anchorOrigin={{
                    vertical: "bottom",
                    horizontal: "right",
                }}
                keepMounted
                transformOrigin={{
                    vertical: "top",
                    horizontal: "right",
                }}
                sx={{maxWidth: 620}}
                open={Boolean(anchorEl)}
                onClose={handleClose}
            >
                {emlDocumentId ? (
                    <MenuItem
                        key={emlDocumentId}
                        onClick={onMenuItem(emlDocumentId, ETransmissionFile.EML)}
                    >
                        {item(
                            String(
                                t("tally.transmissionPackage.actions.download.emlTitle", {
                                    date: lastDocumentDate,
                                })
                            ),
                            String(
                                t("tally.transmissionPackage.actions.download.emlTitle", {
                                    date: lastDocumentTime,
                                })
                            )
                        )}
                    </MenuItem>
                ) : null}
                {csvDocumentId ? (
                    <MenuItem
                        key={csvDocumentId}
                        onClick={onMenuItem(csvDocumentId, ETransmissionFile.CSV)}
                    >
                        {item(
                            String(
                                t("tally.transmissionPackage.actions.download.csvTitle", {
                                    date: lastDocumentTime,
                                })
                            )
                        )}
                    </MenuItem>
                ) : null}
                {packageDocumentId ? (
                    <MenuItem
                        key={packageDocumentId}
                        onClick={onMenuItem(packageDocumentId, ETransmissionFile.PACKAGE)}
                    >
                        {item(
                            String(
                                t(
                                    "tally.transmissionPackage.actions.download.transmissionPackageTitle",
                                    {
                                        date: lastDocumentTime,
                                    }
                                )
                            )
                        )}
                    </MenuItem>
                ) : null}
                <MenuItem key={"report"} onClick={onDownloadReport}>
                    {item(
                        String(
                            t("tally.transmissionPackage.actions.download.transmissionReportTitle")
                        )
                    )}
                </MenuItem>
                {ovcsPublicKey ? (
                    <MenuItem key={"ovcs-public-key"} onClick={onDownloadOvcsKey}>
                        {item(
                            String(
                                t("tally.transmissionPackage.actions.download.ovcsPublicKeyTitle")
                            )
                        )}
                    </MenuItem>
                ) : null}
            </Menu>
            <Dialog
                variant="info"
                open={!!pending}
                ok={String(t("tally.transmissionPackage.actions.download.dialog.confirm"))}
                cancel={String(t("tally.transmissionPackage.actions.download.dialog.cancel"))}
                title={String(t("tally.transmissionPackage.actions.download.dialog.title"))}
                handleClose={onConfirm}
            >
                {t("tally.transmissionPackage.actions.download.dialog.description", {
                    name: areaName,
                })}
            </Dialog>
        </Box>
    )
}
