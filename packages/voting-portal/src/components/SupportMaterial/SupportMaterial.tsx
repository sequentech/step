// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button} from "@mui/material"
import React from "react"
import {useTranslation} from "react-i18next"
import {Dialog, SupportMaterialCard} from "@sequentech/ui-essentials"
import {downloadBlob} from "@sequentech/ui-core"
import DescriptionIcon from "@mui/icons-material/Description"
import {GET_DOCUMENT} from "../../queries/GetDocument"
import {useQuery} from "@apollo/client/react"
import {useGetPublicDocumentUrl} from "../../hooks/public-document-url"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {useAppSelector} from "../../store/hooks"
import {selectDocumentById} from "../../store/documents/documentsSlice"

export interface SupportMaterialProps {
    title: string
    subtitle?: string
    kind: string
    tenantId: string
    documentId: string
    onViewed?: () => void
}

export const SupportMaterial: React.FC<SupportMaterialProps> = ({
    title,
    subtitle,
    kind,
    tenantId,
    documentId,
    onViewed,
}) => {
    const {t} = useTranslation()
    const [openPreview, openPreviewSet] = React.useState<boolean>(false)
    const {getDocumentUrl} = useGetPublicDocumentUrl()
    const videoRef = React.useRef<HTMLIFrameElement>(null)

    const imageData = useAppSelector(selectDocumentById(String(documentId)))

    const handleOpenDialog = async (type: string) => {
        openPreviewSet(true)
    }

    let documentName = imageData?.name
    const documentUrl = documentName ? getDocumentUrl(documentId, documentName) : ""

    const handleDownload = async () => {
        if (!documentUrl || !documentName) {
            return
        }
        try {
            const response = await fetch(documentUrl)
            const blob = await response.blob()
            await downloadBlob(blob, documentName)
        } catch (error) {
            console.error("Error downloading document:", error)
        }
    }

    return (
        <>
            {/* The row itself is `SupportMaterialCard` in `ui-essentials`, so the
                Election Architect's preview shows the same card. What stays here
                is what needs the store and the document URL: the preview dialog
                below, and the click that opens it. */}
            <SupportMaterialCard
                title={title}
                subtitle={subtitle}
                kind={kind}
                onOpen={() => handleOpenDialog("video")}
                openLabel={t("a11y.previewMaterial", {title})}
            />

            <Dialog
                className="support-material-preview-dialog"
                variant="info"
                open={openPreview}
                ok={t("materials.common.close")}
                title={t("materials.common.preview")}
                handleClose={(result: boolean) => {
                    openPreviewSet(false)
                    onViewed?.()
                }}
                fullWidth
                maxWidth="lg"
                expandable
            >
                <Box
                    className="support-material-preview"
                    sx={{
                        display: "flex",
                        flexDirection: "column",
                        gap: "16px",
                        width: "100%",
                        height: "80vh",
                        justifyContent: "center",
                        alignItems: "center",
                    }}
                >
                    <Box
                        className="support-material-preview-content"
                        sx={{
                            display: "flex",
                            flexDirection: "column",
                            justifyContent: "center",
                            alignItems: "center",
                            width: "100%",
                            height: "100%",
                        }}
                    >
                        {kind.includes("image") ? (
                            <img
                                className="support-material-image"
                                src={documentUrl}
                                alt={`tenant-${tenantId}/document-${documentId}/${documentName}`}
                                style={{maxWidth: "100%", maxHeight: "100%", objectFit: "contain"}}
                            />
                        ) : kind.includes("pdf") ? (
                            <Box
                                className="support-material-pdf-container"
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    width: "100%",
                                    height: "100%",
                                }}
                            >
                                <iframe
                                    className="support-material-pdf"
                                    src={documentUrl}
                                    title={`${t(
                                        "materials.common.label"
                                    )} tenant-${tenantId}/document-${documentId}/${documentName}`}
                                    width="100%"
                                    height="100%"
                                    style={{border: "none"}}
                                ></iframe>
                            </Box>
                        ) : kind.includes("video") ? (
                            <Box
                                className="support-material-video-container"
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    width: "100%",
                                    height: "100%",
                                }}
                            >
                                <iframe
                                    className="support-material-video"
                                    ref={videoRef}
                                    width="100%"
                                    height="100%"
                                    src={documentUrl}
                                    title={`${t(
                                        "materials.common.label"
                                    )} tenant-${tenantId}/document-${documentId}/${documentName}`}
                                    referrerPolicy="origin"
                                    sandbox="allow-scripts allow-same-origin"
                                    allow="autoplay;"
                                    style={{border: "none"}}
                                ></iframe>
                            </Box>
                        ) : kind.includes("audio") ? (
                            <Box
                                className="support-material-audio-container"
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    width: "100%",
                                }}
                            >
                                <iframe
                                    className="support-material-audio"
                                    loading="lazy"
                                    width="100%"
                                    height="120"
                                    src={documentUrl}
                                    title={`${t(
                                        "materials.common.label"
                                    )} tenant-${tenantId}/document-${documentId}/${documentName}`}
                                    allow="autoplay"
                                    style={{border: "none"}}
                                ></iframe>
                            </Box>
                        ) : (
                            <Box
                                className="support-material-download-container"
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    gap: "16px",
                                }}
                            >
                                <DescriptionIcon
                                    className="support-material-document-icon"
                                    sx={{fontSize: "5rem"}}
                                />
                                <Button
                                    className="support-material-download-button"
                                    sx={{padding: "10px 24px", minWidth: "unset"}}
                                    variant="secondary"
                                    onClick={handleDownload}
                                >
                                    {t("materials.common.download")}
                                </Button>
                            </Box>
                        )}
                    </Box>
                </Box>
            </Dialog>
        </>
    )
}
