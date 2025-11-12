// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, Typography} from "@mui/material"
import React, {useContext} from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Dialog, theme} from "@sequentech/ui-essentials"
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
            <BorderBox role="button" tabIndex={0}>
                <Box>
                    {kind.includes("image") ? (
                        <ImageIcon sx={{fontSize: "42px", marginRight: "16px"}} />
                    ) : kind.includes("pdf") ? (
                        <PictureAsPdfIcon sx={{fontSize: "42px", marginRight: "16px"}} />
                    ) : kind.includes("video") ? (
                        <VideoFileIcon sx={{fontSize: "42px", marginRight: "16px"}} />
                    ) : kind.includes("audio") ? (
                        <AudioFileIcon sx={{fontSize: "42px", marginRight: "16px"}} />
                    ) : (
                        <DescriptionIcon sx={{fontSize: "42px", marginRight: "16px"}} />
                    )}
                </Box>
                <TextContainer>
                    <StyledTitle>{title}</StyledTitle>
                    <StyledSubTitle>{subtitle}</StyledSubTitle>
                </TextContainer>
                <Box sx={{display: "flex", alignItems: "center"}}>
                    <StyledButton
                        sx={{marginRight: "16px"}}
                        variant="secondary"
                        onClick={() => handleOpenDialog("video")}
                    >
                        <VisibilityIcon />
                    </StyledButton>
                </Box>
            </BorderBox>

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
