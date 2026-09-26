// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import AudioFileIcon from "@mui/icons-material/AudioFile"
import DescriptionIcon from "@mui/icons-material/Description"
import ImageIcon from "@mui/icons-material/Image"
import PictureAsPdfIcon from "@mui/icons-material/PictureAsPdf"
import VideoFileIcon from "@mui/icons-material/VideoFile"
import VisibilityIcon from "@mui/icons-material/Visibility"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import React from "react"

import {stringToHtml} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import PageLimit from "../components/PageLimit/PageLimit"
import {theme} from "../services/theme"

const BorderBox = styled(Box)`
    display: flex;
    flex-direction: row;
    border: 2px solid ${theme.palette.brandSuccess};
    background-color: ${theme.palette.lightBackground};
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

const OpenButton = styled(Button)`
    padding: 10px 24px;
    min-width: unset;
`

const CardTitle = styled(Typography)`
    font-size: 24px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    font-weight: bold;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

const CardSubTitle = styled(Typography)<{component?: React.ElementType}>`
    font-size: 18px;
    line-height: 20px;
    margin-top: 0;
    margin-bottom: 10px;
    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        margin-bottom: 0;
    }
`

const MaterialsList = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 30px;
    margin-bottom: 30px;
`

const Heading = styled(Typography)<{component?: React.ElementType}>`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

export interface ISupportMaterialCardProps {
    /** The document's own name. */
    title: string
    subtitle?: string
    /**
     * The platform's own MIME-ish `kind`, matched by substring.
     *
     * `image`, `pdf`, `video` and `audio` each get their own icon and anything
     * else gets the generic document. Substring rather than equality because the
     * value arrives as a full content type — `application/pdf`, `video/mp4`.
     */
    kind: string
    /** Opening the document. Omit and the button is not drawn. */
    onOpen?: () => void
    openLabel?: string
}

const iconFor = (kind: string): React.JSX.Element => {
    const style = {fontSize: "42px", marginRight: "16px"}
    if (kind.includes("image")) {
        return <ImageIcon className="support-material-image-icon" sx={style} />
    }
    if (kind.includes("pdf")) {
        return <PictureAsPdfIcon className="support-material-pdf-icon" sx={style} />
    }
    if (kind.includes("video")) {
        return <VideoFileIcon className="support-material-video-icon" sx={style} />
    }
    if (kind.includes("audio")) {
        return <AudioFileIcon className="support-material-audio-icon" sx={style} />
    }
    return <DescriptionIcon className="support-material-document-icon" sx={style} />
}

export const SupportMaterialCard: React.FC<ISupportMaterialCardProps> = ({
    title,
    subtitle,
    kind,
    onOpen,
    openLabel,
}) => (
    // Not a button itself: the open button inside is the one control, so the card
    // is a single keyboard stop.
    <BorderBox className="support-material">
        <Box className="support-material-summary">{iconFor(kind)}</Box>
        <TextContainer className="support-material-text">
            <CardTitle className="support-material-title">{title}</CardTitle>
            <CardSubTitle className="support-material-subtitle" component="div">
                {stringToHtml(subtitle || "")}
            </CardSubTitle>
        </TextContainer>
        {onOpen === undefined ? null : (
            <Box className="support-material-actions" sx={{display: "flex", alignItems: "center"}}>
                <OpenButton
                    className="support-material-preview-button"
                    sx={{marginRight: "16px"}}
                    variant="secondary"
                    aria-label={openLabel}
                    onClick={onOpen}
                >
                    <VisibilityIcon className="support-material-preview-icon" />
                </OpenButton>
            </Box>
        )}
    </BorderBox>
)

export interface ISupportMaterialsLayoutProps {
    steps?: React.ReactNode
    /**
     * The event's own heading for this tab, per language.
     *
     * Data rather than wording: it comes from the event's presentation, and the portal
     * passes `"-"` where an event has none. Omitted entirely — which is what a preview of
     * an event that has not named the tab does — the portal's own
     * `materials.common.label` is used, rather than a copy of that word in the caller.
     */
    title?: string
    subtitle?: React.ReactNode
    /** The Back control, which knows where back is and so belongs to the host. */
    back?: React.ReactNode
    /** The cards, or whatever a host wants where the cards go. */
    children?: React.ReactNode
}

/**
 * The tab of documents a voter may want beside the ballot.
 *
 * The third of the portal's screens lifted so the wizard's preview can show it
 * rather than describe it — see {@link ReviewLayout} for the argument. The
 * simplest of the three: a heading, a subtitle, a way back, and a column of
 * cards. What made it worth doing at all is that the cards were not simple.
 */
export const SupportMaterialsLayout: React.FC<ISupportMaterialsLayoutProps> = ({
    steps,
    title,
    subtitle,
    back,
    children,
}) => {
    const {t} = useTranslation()

    return (
        <PageLimit className="support-materials-screen screen" maxWidth="lg">
            {steps === undefined ? null : (
                <Box className="stepper-box" marginTop="48px">
                    {steps}
                </Box>
            )}
            <Box
                className="support-materials-header"
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    minHeight: "100px",
                }}
            >
                <Box className="support-materials-heading">
                    <Heading className="screen-title" variant="h1">
                        <Box className="screen-title-text">
                            {title ?? t("materials.common.label")}
                        </Box>
                    </Heading>
                    {subtitle === undefined ? null : (
                        <Typography
                            className="screen-description"
                            variant="body1"
                            component="div"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {subtitle}
                        </Typography>
                    )}
                </Box>
                {back}
            </Box>
            <MaterialsList className="support-materials-list">{children}</MaterialsList>
        </PageLimit>
    )
}
