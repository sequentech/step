// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import {faCheck, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import React from "react"

import IconButton from "../components/IconButton/IconButton"
import PageLimit from "../components/PageLimit/PageLimit"
import QRCode from "../components/QRCode/QRCode"
import {theme} from "../services/theme"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const BallotIdContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 30px;
    margin: 25px 0;
    align-items: center;
`

const BallotIdBorder = styled(Box)`
    background-color: ${({theme}) => theme.palette.green.light};
    color: ${({theme}) => theme.palette.customGrey.contrastText};
    padding: 10px 12px;
    display: flex;
    flex-direction: row;
    justify-content: left;
    align-items: center;
    gap: 10px;
    border-radius: 4px;
`

/**
 * A plain anchor, not the themed one.
 *
 * The shared theme sets `MuiLink.defaultProps.component = LinkBehavior`, which
 * maps every `href` onto react-router's `to` — so an MUI `Link` cannot render at
 * all without a Router above it, and the Election Architect's preview has none.
 * Discovered by this component's first test failing on *"Cannot destructure
 * property 'basename' of React.useContext(...) as it is null"*.
 *
 * A plain `a` is also the more correct of the two here whatever renders it: the
 * ballot tracker is an absolute URL on another origin opened in a new tab, which
 * is not a route this application has. `styled(Link)` with `component="a"` was
 * the first attempt and does not typecheck — `styled` drops the polymorphic
 * `component` prop — so the element is plain from the start.
 */
const BallotIdLink = styled("a")`
    color: ${({theme}) => theme.palette.brandColor};
    text-decoration: none;
    font-weight: normal;
    overflow-wrap: anywhere;
    text-overflow: ellipsis;
    &:hover {
        text-decoration: underline;
    }
`

const QRContainer = styled(Box)`
    display: flex;
    justify-content: center;
    width: 100%;
    margin: 15px auto;
`

export interface IConfirmationLayoutProps {
    /** The breadcrumb, built by whoever knows how many steps there are. */
    steps?: React.ReactNode

    title: string
    onTitleHelp?: () => void
    description?: React.ReactNode
    /**
     * A second paragraph under the description, when there is something to add.
     *
     * The portal says here that a ballot was cast blank. Its own slot rather than
     * more `description`, because that one is wrapped in a `Typography` and a
     * paragraph inside a paragraph is not markup a browser will keep.
     */
    note?: React.ReactNode

    /** What the identifier is called — *Ballot ID*, in the portal's wording. */
    ballotIdLabel?: string
    /**
     * The identifier itself, as the wide layout shows it.
     *
     * There is a second, narrow rendering because the raw hash does not fit on a
     * phone; the portal passes a sentence there and the bare hash here.
     */
    ballotId: string
    ballotIdOnPhone?: string
    /** Where the identifier links to, or nothing to render it as plain text. */
    ballotIdHref?: string
    onBallotIdClick?: React.MouseEventHandler
    onBallotIdHelp?: () => void

    verifyTitle?: string
    verifyDescription?: React.ReactNode
    /** What the QR encodes. Omit to leave the block out entirely. */
    qrValue?: string

    /** Print, go back, vote in another election. */
    actions?: React.ReactNode

    /** The host's dialogs, which belong with the state that opens them. */
    children?: React.ReactNode
}

/**
 * What a voter sees once their ballot is in, with nothing about how it got there.
 *
 * The sibling of {@link ReviewLayout} and extracted for the same reason: the
 * Election Architect's preview should show this screen rather than a drawing of
 * it, and `ConfirmationScreen` is 540 lines bound to nine store selectors and
 * five GraphQL hooks — none of which a preview has or should have.
 *
 * **The honest caveat lives with the caller, not here.** A confirmation screen's
 * two most prominent values — the ballot identifier and the QR of its tracker
 * URL — are produced by the act of casting, so before an election runs there is
 * no true value for either. This component draws what it is given; whether what
 * it is given is real, and whether the page says so, is the caller's business.
 * The wizard's preview labels them as samples for exactly this reason.
 */
export const ConfirmationLayout: React.FC<IConfirmationLayoutProps> = ({
    steps,
    title,
    onTitleHelp,
    description,
    note,
    ballotIdLabel,
    ballotId,
    ballotIdOnPhone,
    ballotIdHref,
    onBallotIdClick,
    onBallotIdHelp,
    verifyTitle,
    verifyDescription,
    qrValue,
    actions,
    children,
}) => (
    <PageLimit maxWidth="lg" className="confirmation-screen screen">
        {steps === undefined ? null : <Box marginTop="24px">{steps}</Box>}
        <StyledTitle variant="h4" fontSize="24px" fontWeight="bold" sx={{marginTop: "40px"}}>
            <Box>{title}</Box>
            {onTitleHelp === undefined ? null : (
                <IconButton
                    icon={faCircleQuestion}
                    sx={{
                        fontSize: "unset",
                        lineHeight: "unset",
                        paddingBottom: "2px",
                    }}
                    fontSize="16px"
                    onClick={onTitleHelp}
                />
            )}
        </StyledTitle>
        {description === undefined ? null : (
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {description}
            </Typography>
        )}
        {note === undefined ? null : (
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {note}
            </Typography>
        )}
        <BallotIdContainer>
            {ballotIdLabel === undefined ? null : (
                <Typography
                    variant="h5"
                    fontSize="18px"
                    fontWeight="bold"
                    sx={{display: {xs: "none", sm: "block"}}}
                >
                    {ballotIdLabel}
                </Typography>
            )}
            <BallotIdBorder>
                <IconButton
                    icon={faCheck}
                    sx={{
                        fontSize: "unset",
                        lineHeight: "unset",
                        paddingBottom: "2px",
                    }}
                    fontSize="14px"
                    color={theme.palette.customGrey.contrastText}
                />
                <BallotIdLink
                    href={ballotIdHref}
                    target={ballotIdHref === undefined ? undefined : "_blank"}
                    sx={{display: {xs: "none", sm: "block"}}}
                    onClick={onBallotIdClick}
                >
                    {ballotId}
                </BallotIdLink>
                <BallotIdLink
                    href={ballotIdHref}
                    target={ballotIdHref === undefined ? undefined : "_blank"}
                    sx={{display: {xs: "block", sm: "none"}}}
                    onClick={onBallotIdClick}
                >
                    {ballotIdOnPhone ?? ballotId}
                </BallotIdLink>
                {onBallotIdHelp === undefined ? null : (
                    <IconButton
                        icon={faCircleQuestion}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            marginLeft: "16px",
                        }}
                        fontSize="18px"
                        onClick={onBallotIdHelp}
                    />
                )}
                {children}
            </BallotIdBorder>
        </BallotIdContainer>
        {verifyTitle === undefined ? null : (
            <Typography variant="h5" fontSize="18px" fontWeight="bold">
                {verifyTitle}
            </Typography>
        )}
        {verifyDescription === undefined ? null : (
            <Typography
                variant="body2"
                sx={{color: theme.palette.customGrey.main}}
                id="qr-code-description"
            >
                {verifyDescription}
            </Typography>
        )}
        {qrValue === undefined ? null : (
            <QRContainer className="qr-container">
                <QRCode ariaLabelledby="qr-code-description" value={qrValue} />
            </QRContainer>
        )}
        {actions}
    </PageLimit>
)
