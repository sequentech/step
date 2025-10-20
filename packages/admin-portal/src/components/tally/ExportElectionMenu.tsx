// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, CircularProgress, Menu, MenuItem} from "@mui/material"
import React, {useCallback, useContext, useState} from "react"
import {useTranslation} from "react-i18next"
import {EXPORT_FORMATS} from "./constants"
import {FetchDocumentQuery} from "@/gql/graphql"
import styled from "@emotion/styled"
import {theme} from "@sequentech/ui-essentials"
import {downloadUrl} from "@sequentech/ui-core"
import {EExportFormat, IResultDocuments} from "@/types/results"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {MiruExport} from "../MiruExport"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ETallyType, ETallyTypeCssClass} from "@/types/ceremonies"
import {notDeepEqual} from "assert"
import {StyledAppAtom} from "@/App"
import {ETemplateType} from "@/types/templates"
import {GenerateReport} from "./GenerateReport"
import {GeneratePDF} from "./GeneratePdf"
import {GenerateResultsXlsx} from "./GenerateResultsXlsx"

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

export interface IResultDocumentsData {
    documents: IResultDocuments
    name: string
    class_type: string
    class_subtype?: string
}

interface ExportElectionMenuProps {
    buttonTitle?: string
    documentsList: IResultDocumentsData[] | null
    electionEventId: string
    tallySessionId: string
    itemName: string
    tallyType?: string | null
    electionId?: string | null
    miruExportloading?: boolean
    onCreateTransmissionPackage?: (v: {area_id: string; election_id: string}) => void
    tenantId?: string | null
    resultsEventId?: string | null
}

export const ExportElectionMenu: React.FC<ExportElectionMenuProps> = (props) => {
    const {
        itemName,
        documentsList,
        tallySessionId,
        electionEventId,
        buttonTitle,
        tallyType,
        electionId,
        miruExportloading,
        onCreateTransmissionPackage,
        tenantId,
        resultsEventId,
    } = props
    const {globalSettings} = useContext(SettingsContext)
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const [performDownload, setPerformDownload] = useState<IDocumentData | null>(null)
    const handleMenu = (event: React.MouseEvent<HTMLElement>) => {
        event.preventDefault()
        event.stopPropagation()
        setAnchorEl(event.currentTarget)
    }

    const handleClose = useCallback(() => {
        console.log("closing menu")
        setAnchorEl(null)
    }, [])

    const handleExport = (documents: IResultDocuments, format: EExportFormat) => {
        let documentId = documents?.[format]
        if (!documentId) {
            console.log("handleExport ERROR missing document id")
            return
        }

        // If the requested format is tar_gz, check if a tar_gz_pdfs version exists.
        // If it does, use it as the primary download source.
        if (format === EExportFormat.TAR_GZ && documents?.tar_gz_pdfs) {
            documentId = documents.tar_gz_pdfs
        }

        console.log("handleExport setPerformDownload")
        if (format === EExportFormat.RECEIPTS_PDF) {
            setPerformDownload({
                id: documentId,
                kind: EExportFormat.PDF,
                name: `vote_receipts.pdf`,
            })
        } else {
            let extension = format.replace("_", ".") // for converting tar_gz to tar.gz
            setPerformDownload({
                id: documentId,
                kind: format,
                name: `report.${extension}`,
            })
        }
    }

    const isExportFormatDisabled = (documents: IResultDocuments, format: EExportFormat): boolean =>
        !documents?.[format]

    const getMenuClassName = (
        format: EExportFormat,
        classType: string,
        classSubtype?: string
    ): string => {
        let classes: Array<string> = ["tally-document-item", format, classType]

        if (classSubtype) {
            classes.push(classSubtype)
        }
        if (tallyType) {
            let tally_type_class = ""
            switch (tallyType) {
                case ETallyType.ELECTORAL_RESULTS:
                    tally_type_class = ETallyTypeCssClass[ETallyType.ELECTORAL_RESULTS]
                    break
                case ETallyType.INITIALIZATION_REPORT:
                    tally_type_class = ETallyTypeCssClass[ETallyType.INITIALIZATION_REPORT]
                    break
            }
            classes.push(tally_type_class)
        }

        return classes.join(" ")
    }

    return (
        <div key={itemName}>
            <ExportButton
                aria-label="export election data"
                aria-controls="export-menu"
                aria-haspopup="true"
                onClick={handleMenu}
            >
                <span title={buttonTitle ?? t("common.label.actions")}>
                    {buttonTitle ?? t("common.label.actions")}
                </span>
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
                <StyledAppAtom>
                    {documentsList?.map((documents) => (
                        <React.Fragment key={documents.class_type + documents.name}>
                            {EXPORT_FORMATS.map((format) =>
                                isExportFormatDisabled(documents.documents, format.value) ? null : (
                                    <React.Fragment
                                        key={`${documents.class_type}:${documents.name}:${format.value}`}
                                    >
                                        <MenuItem
                                            className={getMenuClassName(
                                                format.value,
                                                documents.class_type,
                                                documents.class_subtype
                                            )}
                                            key={format.value}
                                            onClick={(e: React.MouseEvent<HTMLElement>) => {
                                                e.preventDefault()
                                                e.stopPropagation()
                                                setTimeout(() => handleClose(), 0)
                                                handleExport(documents.documents, format.value)
                                            }}
                                            disabled={isExportFormatDisabled(
                                                documents.documents,
                                                format.value
                                            )}
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
                                                        item: documents.name,
                                                        format: format.label,
                                                    })}
                                                </span>
                                            </Box>
                                        </MenuItem>
                                        {format.value === EExportFormat.HTML ? (
                                            <GeneratePDF
                                                key={documents.name}
                                                documents={documents.documents}
                                                name={documents.name}
                                                electionEventId={electionEventId}
                                                tallySessionId={tallySessionId}
                                                handleClose={handleClose}
                                            />
                                        ) : null}
                                    </React.Fragment>
                                )
                            )}
                        </React.Fragment>
                    ))}
                    {globalSettings?.ACTIVATE_MIRU_EXPORT &&
                    tallyType !== ETallyType.INITIALIZATION_REPORT &&
                    onCreateTransmissionPackage &&
                    electionId ? (
                        <MiruExport
                            handleClose={handleClose}
                            electionId={electionId}
                            onCreateTransmissionPackage={onCreateTransmissionPackage}
                            loading={miruExportloading}
                        />
                    ) : null}
                    {globalSettings?.ACTIVATE_MIRU_EXPORT &&
                    tallyType !== ETallyType.INITIALIZATION_REPORT &&
                    electionId ? (
                        <>
                            <GenerateReport
                                handleClose={handleClose}
                                reportType={ETemplateType.BALLOT_IMAGES}
                                electionEventId={electionEventId}
                                electionId={electionId}
                                tallySessionId={tallySessionId}
                            />
                            <GenerateReport
                                handleClose={handleClose}
                                reportType={ETemplateType.VOTE_RECEIPT}
                                electionEventId={electionEventId}
                                electionId={electionId}
                                tallySessionId={tallySessionId}
                            />
                        </>
                    ) : null}
                    {tenantId &&
                        resultsEventId &&
                        electionEventId &&
                        documentsList &&
                        documentsList.length > 0 &&
                        documentsList[0].class_type === "event" && (
                            <GenerateResultsXlsx
                                eventName={itemName}
                                electionEventId={electionEventId}
                                tallySessionId={tallySessionId}
                                tenantId={tenantId}
                                handleClose={handleClose}
                                resultsEventId={resultsEventId}
                            />
                        )}
                </StyledAppAtom>
            </Menu>
        </div>
    )
}
