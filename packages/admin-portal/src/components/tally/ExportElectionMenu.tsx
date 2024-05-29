// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, CircularProgress, Menu, MenuItem} from "@mui/material"
import Button from "@mui/material/Button"
import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {EXPORT_FORMATS} from "./constants"
import {
    FetchDocumentQuery,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Results_Election,
    Sequent_Backend_Tally_Session,
} from "@/gql/graphql"
import styled from "@emotion/styled"
import {downloadUrl, isNull, theme} from "@sequentech/ui-essentials"
import {useGetList} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {EExportFormat, IResultDocuments} from "@/types/results"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"

interface PerformDownloadProps {
    onDownload: () => void
    fileName: string
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

        downloadUrl(data.fetchDocument.url, fileName).then(() => onDownload())
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

interface IDocumentData {
    id: string
    kind: EExportFormat
    name: string
}

interface ExportElectionMenuProps {
    documents: IResultDocuments | null
    electionEventId: string
    itemName: string
}

export const ExportElectionMenu: React.FC<ExportElectionMenuProps> = (props) => {
    const {itemName, documents, electionEventId} = props
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

    const handleExport = (format: EExportFormat) => {
        let documentId = documents?.[format]
        if (!documentId) {
            console.log("handleExport ERROR missing document id")
            return
        }

        console.log("handleExport setPerformDownload")
        if (format === EExportFormat.RECEIPTS_PDF) {
            setPerformDownload({
                id: documentId,
                kind: EExportFormat.PDF,
                name: `vote_receipts.pdf`,
            })
        } else {
            setPerformDownload({
                id: documentId,
                kind: format,
                name: `report.${format}`,
            })
        }
    }

    const exportFormatItem = itemName /*election
        ? election?.name?.slice(0, 12)
        : contest
        ? contest?.name?.slice(0, 12)
        : area && area !== "all"
        ? areaName?.slice(0, 12)
        : area
        ? t("common.label.globalAreaResults")
        : t("common.label.allResults")*/
    /*
    if (election) {
	    election?.name?.slice(0, 12)
    } else {
        if (contest?.name?.slice(0, 12)) {
        } else {
            if (area && area !== "all") {
                areaName?.slice(0, 12)
            } else {
                if (area) {
                    t("common.label.globalAreaResults")
                } else {
                    t("common.label.allResults")
                }
            }
        }
    } 
    */

    const isExportFormatDisabled = (format: EExportFormat): boolean => !documents?.[format]

    return (
        <div>
            <ExportButton
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                <span title={t("common.label.export")}>{t("common.label.export")}</span>
                {performDownload ? (
                    <PerformDownload
                        onDownload={() => {
                            downloading = false
                            setPerformDownload(null)
                        }}
                        fileName={performDownload.name}
                        documentId={performDownload.id}
                        electionEventId={electionEventId}
                    />
                ) : null}
            </ExportButton>

            <Menu
                id="menu-appbar"
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
                {EXPORT_FORMATS.map((format) =>
                    isExportFormatDisabled(format.value) ? null : (
                        <MenuItem
                            key={format.value}
                            onClick={(e: React.MouseEvent<HTMLElement>) => {
                                e.preventDefault()
                                e.stopPropagation()
                                handleClose()
                                handleExport(format.value)
                            }}
                            disabled={isExportFormatDisabled(format.value)}
                        >
                            <Box
                                sx={{
                                    textOverflow: "ellipsis",
                                    whiteSpace: "nowrap",
                                    overflow: "hidden",
                                }}
                            >
                                <span title={format.label}>
                                    {t("common.label.exportFormat", {
                                        item: exportFormatItem,
                                        format: format.label,
                                    })}
                                </span>
                            </Box>
                        </MenuItem>
                    )
                )}
            </Menu>
        </div>
    )
}
