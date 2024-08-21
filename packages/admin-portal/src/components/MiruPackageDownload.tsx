// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, CircularProgress, Menu, MenuItem} from "@mui/material"
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {FetchDocumentQuery} from "@/gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {downloadUrl} from "@sequentech/ui-core"
import {EExportFormat} from "@/types/results"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {IMiruDocument} from "@/types/miru"
import {TallyStyles} from "@/components/styles/TallyStyles"
import DownloadIcon from "@mui/icons-material/Download"

interface PerformDownloadProps {
    onDownload: () => void
    fileName?: string
    documentId: string
    electionEventId: string
}

let downloading = false

const PerformDownload: React.FC<PerformDownloadProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
}) => {
    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
    })

    if (!loading && !error && data?.fetchDocument?.url && !downloading) {
        downloading = true

        let documentName
        documentName =
            fileName ??
            (() => {
                // data.sequent_backend_document.name + getDocumentExtension(data.sequent_backend_document.mediaType)//to be enabled after generating updated hasura types
                return "document" + ".xml"
            })()

        downloadUrl(data.fetchDocument.url, documentName).then(() => onDownload())
    }

    return <CircularProgress />
}

interface MiruPackageDownloadProps {
    documents: IMiruDocument[] | null
    areaName: string | null | undefined
    electionEventId: string
}

interface IDocumentData {
    id: string
    kind: EExportFormat
    name: string
}

export const MiruPackageDownload: React.FC<MiruPackageDownloadProps> = (props) => {
    const {areaName, documents, electionEventId} = props
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [openModal, setOpenModal] = useState(false)
    const [performDownload, setPerformDownload] = useState<IDocumentData | null>(null)
    const [documentToDownload, setDocumentToDownload] = useState<IDocumentData | null>(null)
    const handleMenu = (event: React.MouseEvent<HTMLElement>) => {
        event.preventDefault()
        event.stopPropagation()
        setAnchorEl(event.currentTarget)
    }

    const handleClose = () => {
        setAnchorEl(null)
    }

    const handleDownload = (doc: IDocumentData) => {
        setPerformDownload(doc)
        /*{
            id: doc.document_id,
            kind: EExportFormat.JSON, //need to adjust this to right format because document is currently not readable
            name: `MiruDocument.json`,
        })*/
    }
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
                {performDownload ? (
                    <PerformDownload
                        onDownload={() => {
                            downloading = false
                            setDocumentToDownload(null)
                            setPerformDownload(null)
                        }}
                        // fileName={performDownload.name}
                        documentId={performDownload.id}
                        electionEventId={electionEventId}
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
                {documents?.map((doc) => (
                    <>
                        <MenuItem
                            key={doc.document_id}
                            onClick={(e: React.MouseEvent<HTMLElement>) => {
                                e.preventDefault()
                                e.stopPropagation()
                                handleClose()
                                setDocumentToDownload({
                                    id: doc.document_id,
                                    kind: EExportFormat.JSON, //need to adjust this to right format because document is currently not readable
                                    name: `er_15363610.xz`,
                                })
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
                                <span title={t("tally.transmissionPackage.actions.download.itemTitleER")}>
                                    {t("tally.transmissionPackage.actions.download.itemTitleER")}
                                </span>
                            </Box>
                        </MenuItem>
                        {
                            doc?.servers_sent_to
                                .filter(server => !!server.document_id)
                                .map(server =>
                                <MenuItem
                                    key={server.document_id}
                                    onClick={(e: React.MouseEvent<HTMLElement>) => {
                                        e.preventDefault()
                                        e.stopPropagation()
                                        handleClose()
                                        setDocumentToDownload({
                                            id: server.document_id!,
                                            kind: EExportFormat.JSON, //need to adjust this to right format because document is currently not readable
                                            name: `er_15363610.zip`,
                                        })
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
                                        <span title={t("tally.transmissionPackage.actions.download.itemTitleTR", {
                                            server: server.name,
                                        })}>
                                            {t("tally.transmissionPackage.actions.download.itemTitleTR", {
                                                server: server.name,
                                            })}
                                        </span>
                                    </Box>
                                </MenuItem>
                            )
                        }
                    </>
                ))}
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
                        handleDownload(documentToDownload)
                    }
                    setDocumentToDownload(null)
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
