// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, Typography} from "@mui/material"
import React, {useContext} from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Dialog, theme} from "@sequentech/ui-essentials"
import {downloadBlob} from "@sequentech/ui-core"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {GET_DOCUMENT} from "../../queries/GetDocument"
import {useQuery} from "@apollo/client/react"
import VideoFileIcon from "@mui/icons-material/VideoFile"
import AudioFileIcon from "@mui/icons-material/AudioFile"
import PictureAsPdfIcon from "@mui/icons-material/PictureAsPdf"
import ImageIcon from "@mui/icons-material/Image"
import DescriptionIcon from "@mui/icons-material/Description"
import {useGetPublicDocumentUrl} from "../../hooks/public-document-url"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {useAppSelector} from "../../store/hooks"
import {selectDocumentById} from "../../store/documents/documentsSlice"

const BorderBox = styled(Box)`
    display: flex;
    flex-direction: row;
    border: 2px solid ${theme.palette.brandSuccess};
    background-color: ${theme.palette.lightBackground};
    display: flex;
    flex-direction: row;
    padding: 19px 38px;
    align-items: center;
    gap: 21px;
    color: ${({theme}) => theme.palette.black};

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        position: relative;
        flex-direction: column;
        padding: 27px 18px;
    }
`

const TextContainer = styled(Box)`
    flex-grow: 2;
    text-align: left;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        display: flex;
        flex-direction: row;
        justify-content: space-between;
        width: 100%;
    }
`

const StyledButton = styled(Button)`
    padding: 10px 24px;
    min-width: unset;
`

const StyledTitle = styled(Typography)`
    font-size: 24px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    font-weight: bold;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

const StyledSubTitle = styled(Typography)`
    font-size: 18px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

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
            <BorderBox className="support-material">
                <Box className="support-material-summary">
                    {kind.includes("image") ? (
                        <ImageIcon
                            className="support-material-image-icon"
                            sx={{fontSize: "42px", marginRight: "16px"}}
                        />
                    ) : kind.includes("pdf") ? (
                        <PictureAsPdfIcon
                            className="support-material-pdf-icon"
                            sx={{fontSize: "42px", marginRight: "16px"}}
                        />
                    ) : kind.includes("video") ? (
                        <VideoFileIcon
                            className="support-material-video-icon"
                            sx={{fontSize: "42px", marginRight: "16px"}}
                        />
                    ) : kind.includes("audio") ? (
                        <AudioFileIcon
                            className="support-material-audio-icon"
                            sx={{fontSize: "42px", marginRight: "16px"}}
                        />
                    ) : (
                        <DescriptionIcon
                            className="support-material-document-icon"
                            sx={{fontSize: "42px", marginRight: "16px"}}
                        />
                    )}
                </Box>
                <TextContainer className="support-material-text">
                    <StyledTitle className="support-material-title">{title}</StyledTitle>
                    <StyledSubTitle className="support-material-subtitle">
                        {subtitle}
                    </StyledSubTitle>
                </TextContainer>
                <Box
                    className="support-material-actions"
                    sx={{display: "flex", alignItems: "center"}}
                >
                    <StyledButton
                        className="support-material-preview-button"
                        sx={{marginRight: "16px"}}
                        variant="secondary"
                        onClick={() => handleOpenDialog("video")}
                        aria-label={t("a11y.previewMaterial", {title})}
                    >
                        <VisibilityIcon className="support-material-preview-icon" />
                    </StyledButton>
                </Box>
            </BorderBox>

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
                                    sx={{fontSize: "80px"}}
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
