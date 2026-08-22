// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Typography} from "@mui/material"
import React, {useContext} from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Dialog, SupportMaterialCard, theme} from "@sequentech/ui-essentials"
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
}

export const SupportMaterial: React.FC<SupportMaterialProps> = ({
    title,
    subtitle,
    kind,
    tenantId,
    documentId,
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
            />

            <Dialog
                variant="info"
                open={openPreview}
                ok={t("materials.common.close")}
                title={t("materials.common.preview")}
                handleClose={(result: boolean) => {
                    openPreviewSet(false)
                }}
                fullWidth
            >
                <Box
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
                        sx={{
                            display: "flex",
                            flexDirection: "column",
                            justifyContent: "center",
                            alignItems: "center",
                        }}
                    >
                        {kind.includes("image") ? (
                            <>
                                <img
                                    src={documentUrl}
                                    alt={`tenant-${tenantId}/document-${documentId}/${documentName}`}
                                />
                            </>
                        ) : kind.includes("pdf") ? (
                            <Box
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    width: "100%",
                                }}
                            >
                                <iframe
                                    src={documentUrl}
                                    title={`${t(
                                        "materials.common.label"
                                    )} tenant-${tenantId}/document-${documentId}/${documentName}`}
                                    width="1400"
                                    height="800"
                                ></iframe>
                            </Box>
                        ) : kind.includes("video") ? (
                            <Box
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    width: "100%",
                                }}
                            >
                                <iframe
                                    ref={videoRef}
                                    width="800"
                                    height="500"
                                    src={documentUrl}
                                    title={`${t(
                                        "materials.common.label"
                                    )} tenant-${tenantId}/document-${documentId}/${documentName}`}
                                    referrerPolicy="origin"
                                    sandbox="allow-scripts allow-same-origin"
                                    allow="autoplay;"
                                ></iframe>
                            </Box>
                        ) : kind.includes("audio") ? (
                            <Box
                                sx={{
                                    display: "flex",
                                    flexDirection: "column",
                                    justifyContent: "center",
                                    alignItems: "center",
                                    width: "100%",
                                }}
                            >
                                <iframe
                                    loading="lazy"
                                    width="800"
                                    height="120"
                                    src={documentUrl}
                                    title={`${t(
                                        "materials.common.label"
                                    )} tenant-${tenantId}/document-${documentId}/${documentName}`}
                                    allow="autoplay"
                                ></iframe>
                            </Box>
                        ) : null}
                    </Box>
                </Box>
            </Dialog>
        </>
    )
}
