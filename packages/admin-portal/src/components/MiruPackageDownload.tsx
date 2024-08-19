// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, CircularProgress, Menu, MenuItem} from "@mui/material"
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {FetchDocumentQuery} from "@/gql/graphql"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"
import {downloadUrl} from "@sequentech/ui-core"
import {EExportFormat, IResultDocuments} from "@/types/results"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {IMiruDocument} from "@/types/miru"

interface PerformDownloadProps {
    onDownload: () => void
    fileName?: string
    documentId: string
    electionEventId: string
}

let downloading = false

const getDocumentExtension = (docExtension: "application/xml") => {
    switch (docExtension) {
        case "application/xml":
            return ".xml"
            break

        default:
            break
    }
}

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

export const ExportButton = styled.div`
    cursor: pointer;
    margin-left: 10px;
    margin-right: 10px;
    padding: 5px 10px;
    background-color: transparent;
    color: ${theme.palette.primary.dark};
    font-size: 14px;
    font-weight: 500;
    line-height: 1.5;
    text-align: center;
    text-transform: uppercase;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    &:hover {
        background-color: ${theme.palette.primary.dark};
        color: ${theme.palette.white};
    }
`

interface MiruPackageDownloadProps {
    documents: IMiruDocument[] | null
    electionEventId: string
}

interface IDocumentData {
    id: string
    kind: EExportFormat
    name: string
}

export const MiruPackageDownload: React.FC<MiruPackageDownloadProps> = (props) => {
    const {documents, electionEventId} = props
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [performDownload, setPerformDownload] = useState<IDocumentData | null>(null)
    const handleMenu = (event: React.MouseEvent<HTMLElement>) => {
        event.preventDefault()
        event.stopPropagation()
        setAnchorEl(event.currentTarget)
    }

    const handleClose = () => {
        setAnchorEl(null)
    }

    const handleDownload = (doc: IMiruDocument) => {
        setPerformDownload({
            id: doc.document_id,
            kind: EExportFormat.JSON, //need to adjust this to right format because document is currently not readable
            name: `MiruDocument.json`,
        })
    }
    return (
        <div>
            <ExportButton
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                <span title={"Download"}>{t("auditScreen.downloadButton")}</span>
                {performDownload ? (
                    <PerformDownload
                        onDownload={() => {
                            downloading = false
                            setPerformDownload(null)
                        }}
                        // fileName={performDownload.name}
                        documentId={performDownload.id}
                        electionEventId={electionEventId}
                    />
                ) : null}
            </ExportButton>

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
                {documents!.map((doc) => (
                    <MenuItem
                        key={doc.document_id}
                        onClick={(e: React.MouseEvent<HTMLElement>) => {
                            e.preventDefault()
                            e.stopPropagation()
                            handleClose()
                            handleDownload(doc)
                        }}
                    >
                        <Box
                            sx={{
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                            }}
                        >
                            <span title={"Download Document"}>
                                {t("usersAndRolesScreen.permissions.document-download")}
                            </span>
                        </Box>
                    </MenuItem>
                ))}
            </Menu>
        </div>
    )
}
