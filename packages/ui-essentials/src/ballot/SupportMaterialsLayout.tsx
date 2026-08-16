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

import PageLimit from "../components/PageLimit/PageLimit"
import {theme} from "../services/theme"

const BorderBox = styled(Box)`
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

const CardSubTitle = styled(Typography)`
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

const Heading = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

export interface ISupportMaterialCardProps {
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
        return <ImageIcon sx={style} />
    }
    if (kind.includes("pdf")) {
        return <PictureAsPdfIcon sx={style} />
    }
    if (kind.includes("video")) {
        return <VideoFileIcon sx={style} />
    }
    if (kind.includes("audio")) {
        return <AudioFileIcon sx={style} />
    }
    return <DescriptionIcon sx={style} />
}

/**
 * One document in the support materials list, with nothing about fetching it.
 *
 * Split out of the voting portal's `SupportMaterial`, which is 244 lines and
 * reads a thumbnail out of the store by `document_id` — a thing the Election
 * Architect's preview has no way to do, because the documents in a plan have not
 * been uploaded anywhere yet. What both need is the same row: an icon chosen by
 * kind, a title, a subtitle, and a way in.
 *
 * The button is omitted rather than disabled when there is nothing to open. A
 * disabled control is a promise that it would work under some condition the
 * reader is invited to guess at; in a preview there is no such condition.
 */
export const SupportMaterialCard: React.FC<ISupportMaterialCardProps> = ({
    title,
    subtitle,
    kind,
    onOpen,
    openLabel,
}) => (
    <BorderBox role="button" tabIndex={0}>
        <Box>{iconFor(kind)}</Box>
        <TextContainer>
            <CardTitle>{title}</CardTitle>
            <CardSubTitle>{subtitle}</CardSubTitle>
        </TextContainer>
        {onOpen === undefined ? null : (
            <Box sx={{display: "flex", alignItems: "center"}}>
                <OpenButton
                    sx={{marginRight: "16px"}}
                    variant="secondary"
                    aria-label={openLabel}
                    onClick={onOpen}
                >
                    <VisibilityIcon />
                </OpenButton>
            </Box>
        )}
    </BorderBox>
)

export interface ISupportMaterialsLayoutProps {
    steps?: React.ReactNode
    title: string
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
}) => (
    <PageLimit maxWidth="lg">
        {steps === undefined ? null : <Box marginTop="48px">{steps}</Box>}
        <Box
            sx={{
                display: "flex",
                flexDirection: "row",
                justifyContent: "space-between",
                alignItems: "center",
                minHeight: "100px",
            }}
        >
            <Box>
                <Heading variant="h1">
                    <Box>{title}</Box>
                </Heading>
                {subtitle === undefined ? null : (
                    <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                        {subtitle}
                    </Typography>
                )}
            </Box>
            {back}
        </Box>
        <MaterialsList>{children}</MaterialsList>
    </PageLimit>
)
