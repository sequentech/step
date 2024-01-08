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
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"

const ExportButton = styled.div`
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
    const [tenantId] = useTenantStore()

    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            tenantId,
            electionEventId,
            documentId,
        },
    })

    console.log(`FFFF is downloading ${downloading}`)
    if (!loading && !error && data?.fetchDocument?.url && !downloading) {
        downloading = true
        console.log(`FFFF downloadUrl ${downloading}`)

        downloadUrl(data.fetchDocument.url, fileName).then(() => onDownload())
    }

    return <CircularProgress />
}

interface IDocumentData {
    id: string
    kind: EExportFormat
    name: string
}

interface ExportElectionMenuProps {
    resource: string
    event?: Sequent_Backend_Tally_Session
    election?: Sequent_Backend_Election
    contest?: Sequent_Backend_Contest
    area?: Sequent_Backend_Area_Contest | string | undefined
    areaName?: string | undefined
    resultsEventId: string | null
}

export const ExportElectionMenu: React.FC<ExportElectionMenuProps> = (props) => {
    const {resource, event, election, contest, area, areaName, resultsEventId} = props
    const {t} = useTranslation()
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [documents, setDocuments] = useState<IResultDocuments | null>(null)
    const [performDownload, setPerformDownload] = useState<IDocumentData | null>(null)

    const {data: results} = useGetList<Sequent_Backend_Results_Election>(
        resource,
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                tenant_id: election?.tenant_id,
                election_event_id: election?.election_event_id,
                results_event_id: resultsEventId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    useEffect(() => {
        if (results?.[0]?.documents && isNull(documents)) {
            setDocuments(results?.[0]?.documents ?? null)
        }
    }, [results?.[0]?.documents])

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
            console.log("FFFFF handleExport ERROR missing document id")
            return
        }

        console.log("FFFFF handleExport setPerformDownload")
        setPerformDownload({
            id: documentId,
            kind: format,
            name: `report.${format}`,
        })
    }

    if (election) {
        if 
    }

    const exportFormatItem = election
        ? election?.name?.slice(0, 12)
        : contest
        ? contest?.name?.slice(0, 12)
        : area && area !== "all"
        ? areaName?.slice(0, 12)
        : area
        ? t("common.label.globalAreaResults")
        : t("common.label.allResults")

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
                {performDownload && election?.election_event_id ? (
                    <PerformDownload
                        onDownload={() => {
                            downloading = false
                            setPerformDownload(null)
                        }}
                        fileName={performDownload.name}
                        documentId={performDownload.id}
                        electionEventId={election?.election_event_id}
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
                {EXPORT_FORMATS.map((format) => (
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
                ))}
            </Menu>
        </div>
    )
}
