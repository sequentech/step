// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Menu, MenuItem} from "@mui/material"
import React, {useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {sanitizeFilename, FOLDER_MAX_CHARS} from "@sequentech/ui-core"
import {EExportFormat} from "@/types/results"
import {IMiruDocument} from "@/types/miru"
import {TallyStyles} from "@/components/styles/TallyStyles"
import DownloadIcon from "@mui/icons-material/Download"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {GENERATE_TRANSMISSION_REPORT} from "@/queries/GenerateTransmissionReport"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"

interface MiruPackageDownloadProps {
    documents: IMiruDocument[] | null
    areaName: string | null | undefined
    tenantId: string
    electionEventId: string
    electionId?: string
    tallySessionId?: string
    eventName: string
}

interface IDocumentData {
    id: string
    kind: EExportFormat
    name: string
}

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

export const MiruPackageDownload: React.FC<MiruPackageDownloadProps> = ({
    areaName,
    documents,
    tenantId,
    electionEventId,
    electionId,
    tallySessionId,
    eventName,
}) => {
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [openModal, setOpenModal] = useState(false)
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
        const currWidget = addWidget(ETasksExecution.GENERATE_TRANSMISSION_REPORT)
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

    const lastDocument = useMemo(() => {
        let newestDate = new Date(0)
        let newestDocument = null
        for (let document of documents || []) {
            let date = document.created_at && new Date(document.created_at)
            if (date && date > newestDate) {
                newestDate = date
                newestDocument = document
            }
        }

        return newestDocument
    }, [documents])

    const lastDocumentDate =
        (lastDocument?.created_at && formatDate(new Date(lastDocument?.created_at))) || ""

    const emlDocumentId = lastDocument?.document_ids.eml
    return (
        <Box>
            <TallyStyles.MiruToolbarButton
                variant="outlined"
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                <DownloadIcon />
                <span title={t("tally.transmissionPackage.actions.download.title")}>
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
                        onClick={(e: React.MouseEvent<HTMLElement>) => {
                            e.preventDefault()
                            e.stopPropagation()
                            handleClose()
                            setFileNameWithExt(fileName + lastDocumentDate + ".eml")
                            setDocumentToDownload(emlDocumentId)
                            setOpenModal(true)
                        }}
                    >
                        <Box
                            sx={{
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                            }}
                        >
                            <span title={t("tally.transmissionPackage.actions.download.emlTitle")}>
                                {t("tally.transmissionPackage.actions.download.emlTitle", {
                                    date: lastDocumentDate,
                                })}
                            </span>
                        </Box>
                    </MenuItem>
                ) : null}
                {lastDocument && (
                    <MenuItem
                        key={lastDocument.document_ids.all_servers}
                        onClick={(e: React.MouseEvent<HTMLElement>) => {
                            e.preventDefault()
                            e.stopPropagation()
                            handleClose()
                            setFileNameWithExt(fileName + lastDocumentDate + ".zip")
                            setDocumentToDownload(lastDocument.document_ids.all_servers)
                            setOpenModal(true)
                        }}
                    >
                        <Box
                            sx={{
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                            }}
                        >
                            <span
                                title={t(
                                    "tally.transmissionPackage.actions.download.transmissionPackageTitle",
                                    {
                                        date: lastDocumentDate,
                                    }
                                )}
                            >
                                {t(
                                    "tally.transmissionPackage.actions.download.transmissionPackageTitle",
                                    {
                                        date: lastDocumentDate,
                                    }
                                )}
                            </span>
                        </Box>
                    </MenuItem>
                )}
                <MenuItem key={"report"} onClick={onDownloadReport}>
                    <Box
                        sx={{
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                        }}
                    >
                        <span
                            title={t(
                                "tally.transmissionPackage.actions.download.transmissionReportTitle"
                            )}
                        >
                            {t(
                                "tally.transmissionPackage.actions.download.transmissionReportTitle"
                            )}
                        </span>
                    </Box>
                </MenuItem>
            </Menu>
            <Dialog
                variant="info"
                open={openModal}
                ok={t("tally.transmissionPackage.actions.download.dialog.confirm")}
                cancel={t("tally.transmissionPackage.actions.download.dialog.cancel")}
                title={t("tally.transmissionPackage.actions.download.dialog.title")}
                handleClose={(result: boolean) => {
                    if (!documentToDownload) {
                        console.log("error, documentToDownload is null")
                        return
                    }
                    if (result) {
                        setPerformDownload(true)
                    }
                    setOpenModal(false)
                }}
            >
                {t("tally.transmissionPackage.actions.download.dialog.description", {
                    name: areaName,
                })}
            </Dialog>
        </Box>
    )
}
