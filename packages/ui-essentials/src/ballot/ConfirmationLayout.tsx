// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import Box from "@mui/material/Box"
import Link from "@mui/material/Link"
import Typography from "@mui/material/Typography"
import {styled} from "@mui/material/styles"
import {faCheck, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import React, {useId} from "react"

import {BallotHashCopyButton} from "../components/BallotHash/BallotHash"
import Icon from "../components/Icon/Icon"
import DecorativeIconBox from "../components/Icon/DecorativeIconBox"
import IconButton from "../components/IconButton/IconButton"
import {stringToHtml} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import PageLimit from "../components/PageLimit/PageLimit"
import QRCode from "../components/QRCode/QRCode"
import {theme} from "../services/theme"

/** The design's greys and lines (board ConfirmationV42B). */
const COLORS = {
    body: "#4A5262",
    label: "#5B6474",
    line: "#DCE1EE",
    rule: "#E4E8F2",
    qrLine: "#E4E8F2",
} as const

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

/** The width of the ticket beside the text on a wide screen. */
const TICKET_WIDTH = 404

/** Two columns from here; one, in the phone's order, below. */
const TWO_COLUMNS = theme.breakpoints.up("md")
const PHONE = theme.breakpoints.down("sm")

/**
 * The page under the breadcrumb: the outcome and what to do with the receipt
 * on the left, the receipt itself as a ticket on the right. The ticket comes
 * second in the markup, which is the order a phone shows (title, ticket, keep
 * your receipt, updates); on a wide screen the grid puts it in its own column,
 * across every row, so the left column's rows keep their own heights.
 */
const Columns = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 32px;
    margin: 40px 0 48px;

    ${PHONE} {
        gap: 24px;
        margin: 24px 0;
    }

    ${TWO_COLUMNS} {
        display: grid;
        grid-template-columns: minmax(0, 1fr) ${TICKET_WIDTH}px;
        grid-template-rows: auto auto auto 1fr;
        column-gap: 64px;
        row-gap: 36px;
        align-items: start;
        margin-top: 56px;

        &.confirmation-columns-single {
            grid-template-columns: minmax(0, 1fr);
        }

        & > .confirmation-outcome {
            grid-column: 1;
            grid-row: 1;
        }
        & > .confirmation-ticket {
            grid-column: 2;
            grid-row: 1 / -1;
        }
        & > .confirmation-keep {
            grid-column: 1;
            grid-row: 2;
        }
        & > .confirmation-updates {
            grid-column: 1;
            grid-row: 3;
        }
    }
`

const Outcome = styled(Box)<{component?: React.ElementType}>`
    display: flex;
    align-items: center;
    gap: 18px;
    min-width: 0;

    ${PHONE} {
        align-items: flex-start;
        gap: 14px;
    }
`

const OutcomeIcon = styled("span")`
    flex: none;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background-color: ${({theme}) => theme.palette.green.light};
    color: ${({theme}) => theme.palette.green.dark};
    display: flex;
    align-items: center;
    justify-content: center;

    ${PHONE} {
        width: 44px;
        height: 44px;

        svg {
            width: 22px;
            height: 22px;
        }
    }
`

const Meta = styled("p")`
    margin: 4px 0 0;
    min-height: 22px;
    font-size: 15px;
    line-height: 22px;
    color: ${COLORS.body};
    font-variant-numeric: tabular-nums;

    ${PHONE} {
        min-height: 20px;
        font-size: 14px;
        line-height: 20px;
    }
`

const Section = styled(Box)<{component?: React.ElementType}>`
    min-width: 0;
`

const Lead = styled(Box)<{component?: React.ElementType}>`
    margin: 4px 0 0;
    max-width: 52ch;
    font-size: 15px;
    line-height: 22px;
    color: ${COLORS.body};
`

const Updates = styled(Box)`
    min-width: 0;
    padding-top: 28px;
    border-top: 1px solid ${COLORS.rule};

    ${PHONE} {
        padding-top: 22px;
    }
`

const Ticket = styled(Box)<{component?: React.ElementType}>`
    position: relative;
    min-width: 0;
    border: 1px solid ${COLORS.line};
    border-radius: 16px;
    background-color: #fff;
    box-shadow:
        0 1px 2px rgba(15, 5, 76, 0.05),
        0 18px 36px -22px rgba(15, 5, 76, 0.32);
    overflow: hidden;
`

const TicketHead = styled(Box)`
    padding: 18px 24px 20px;
    background-color: ${({theme}) => theme.palette.brandColor};
    color: #fff;

    ${PHONE} {
        padding: 14px 16px 16px;
    }
`

const TicketBody = styled(Box)`
    padding: 20px 24px 22px;

    ${PHONE} {
        padding: 16px;
    }
`

/** The tear line, with a notch cut out of each edge. */
const Perforation = styled("div")`
    position: relative;
    margin: 0 22px;
    border-top: 2px dashed ${COLORS.line};

    &::before,
    &::after {
        content: "";
        position: absolute;
        top: -12px;
        width: 22px;
        height: 22px;
        box-sizing: border-box;
        border: 1px solid ${COLORS.line};
        border-radius: 50%;
        background-color: #fff;
    }
    &::before {
        left: -35px;
    }
    &::after {
        right: -35px;
    }

    ${PHONE} {
        margin: 0 14px;

        &::before {
            left: -27px;
        }
        &::after {
            right: -27px;
        }
    }
`

const QRFigure = styled("figure")`
    display: flex;
    align-items: center;
    gap: 16px;
    margin: 0;
    padding: 18px 24px 22px;

    svg {
        flex: none;
        display: block;
    }

    ${PHONE} {
        padding: 14px 16px 16px;
    }
`

const BallotIdContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 6px;
`

const BallotIdLine = styled(Box)`
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 12px;
    min-width: 0;
`

const BallotIdBorder = styled(Box)`
    background-color: ${({theme}) => theme.palette.green.light};
    color: ${({theme}) => theme.palette.customGrey.contrastText};
    padding: 6px 8px 6px 6px;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 6px;
    min-width: 0;
    border-radius: 8px;
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
const BallotIdLink = styled(Link)<{component?: React.ElementType}>`
    color: ${({theme}) => theme.palette.brandColor};
    text-decoration: none;
    font-weight: normal;
    overflow-wrap: anywhere;
    &:hover {
        text-decoration: underline;
    }
`

/** The whole hash, for an event without transparency: it wraps on the ticket. */
const FullBallotId = styled("span")`
    display: block;
    font-family: ${MONO};
    font-size: 14px;
    line-height: 20px;
    letter-spacing: 0.02em;
    word-break: break-all;
`

const ShortBallotId = styled("span")`
    font-family: ${MONO};
    font-size: 22px;
    line-height: 30px;
    font-weight: 700;
    letter-spacing: 0.04em;
    white-space: nowrap;
    font-variant-ligatures: none;

    ${PHONE} {
        font-size: 20px;
    }
`

const BallotIdText = styled(Box)`
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
`

/** The ballot box, in a soft green circle: not a check mark, so the status
 * check beside the Tracking ID stays the only check on the page. */
const BallotBoxMark: React.FC = () => (
    <svg
        className="confirmation-outcome-mark"
        width="28"
        height="28"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
    >
        <path className="confirmation-outcome-mark-path" d="M3 12.5h18V20H3z" />
        <path className="confirmation-outcome-mark-path" d="M7 12.5V4h10v8.5" />
        <path className="confirmation-outcome-mark-path" d="M10 7.5h4M10 10h4" />
    </svg>
)

export interface IConfirmationLayoutProps {
    /** The breadcrumb, built by whoever knows how many steps there are. */
    steps?: React.ReactNode

    /**
     * Whether the ballot was cast blank, which the screen says out loud.
     *
     * A flag rather than the sentence: the sentence is
     * `confirmationScreen.blankBallot.description`, which belongs in the catalogue with
     * everything else on this screen. `EA-F3-015`.
     */
    isBlankBallot?: boolean
    /**
     * Every contest was acclaimed, so nothing was cast: the screen says
     * `confirmationScreen.acclamation.*` and has no ticket, identifier, link or QR.
     */
    isFullyAcclaimed?: boolean

    /** The election's name, under the title and on the ticket. */
    electionName?: string
    /** The election event's name, in the ticket's band. */
    eventName?: string
    /**
     * When the ballot was cast, already formatted. Left out until it is known:
     * the line it goes on keeps its height meanwhile.
     */
    castAt?: string

    /**
     * The copy button's wording, which also asks for one beside the identifier. The
     * portal offers it unless the event hides auditing altogether
     * (`EVotingPortalAuditButtonCfg.NOT_SHOW`), and not with transparency, where the
     * full Tracking ID in `ballotIdDetails` has its own.
     */
    ballotIdCopyLabels?: {copy: string; copied: string; error: string}

    /** The identifier itself: the whole of it, unless `shortBallotId` is given. */
    ballotId: string
    /**
     * Where the identifier links to. Without it the identifier is a button when
     * `onBallotIdClick` is given, and plain text otherwise.
     */
    ballotIdHref?: string
    /** Null keeps navigation in the current tab. */
    ballotIdTarget?: React.HTMLAttributeAnchorTarget | null
    onBallotIdClick?: React.MouseEventHandler
    onBallotIdHelp?: () => void
    /**
     * The Tracking ID as a short, grouped code (`FTBE-MHRX`), for events with
     * transparency on. It takes the place of `ballotId` on the ticket, and the tag
     * follows it on the same line. `ballotId` still feeds the tracker link and the QR.
     */
    shortBallotId?: string
    ballotIdTag?: React.ReactNode
    /** What the copy button copies, when it is not `ballotId`. */
    ballotIdCopyValue?: string
    /** The status beside the Tracking ID, in place of the plain check mark. */
    ballotIdStatus?: React.ReactNode
    /** Styling for the buttons beside the ID. */
    ballotIdButtonSx?: Record<string, unknown>
    ballotIdCopiedColor?: string
    /** Under the Tracking ID on the ticket: the portal's full Tracking ID, with transparency. */
    ballotIdDetails?: React.ReactNode

    /** What the QR encodes. Omit to leave it, and the tear line, off the ticket. */
    qrValue?: string

    /** Save receipt and Finish, in the row under "Keep your receipt". */
    actions?: React.ReactNode

    /** The optional section under the actions: the portal's Get email updates. */
    updates?: React.ReactNode

    /** The host's dialogs, which belong with the state that opens them. */
    children?: React.ReactNode
}

/**
 * What a voter sees once their ballot is in, with nothing about how it got there
 * (design v42, "the receipt as a ticket").
 *
 * The outcome first: a ballot-box mark beside the title, with the election and the
 * cast time under it. Then *Keep your receipt*, with the caller's actions in one row,
 * and the caller's optional updates under a rule. The receipt itself is a ticket: a
 * navy band with the event, the election and the cast time; the Tracking ID with its
 * help, status and tag; the caller's details (the full Tracking ID); a tear line; and
 * the QR code.
 *
 * The sibling of {@link ReviewLayout} and extracted for the same reason: the
 * Election Architect's preview should show this screen rather than a drawing of
 * it, and `ConfirmationScreen` is bound to store selectors and GraphQL hooks that a
 * preview has not got.
 *
 * **The honest caveat lives with the caller, not here.** A confirmation screen's
 * two most prominent values — the ballot identifier and the QR of its tracker
 * URL — are produced by the act of casting, so before an election runs there is
 * no true value for either. This component draws what it is given; whether what
 * it is given is real, and whether the page says so, is the caller's business.
 */
export const ConfirmationLayout: React.FC<IConfirmationLayoutProps> = ({
    steps,
    isBlankBallot = false,
    isFullyAcclaimed = false,
    electionName,
    eventName,
    castAt,
    ballotIdCopyLabels,
    ballotId,
    ballotIdHref,
    ballotIdTarget = "_blank",
    onBallotIdClick,
    onBallotIdHelp,
    shortBallotId,
    ballotIdTag,
    ballotIdCopyValue,
    ballotIdStatus,
    ballotIdButtonSx,
    ballotIdCopiedColor,
    ballotIdDetails,
    qrValue,
    actions,
    updates,
    children,
}) => {
    const {t} = useTranslation()
    const ids = useId()
    const titleId = `${ids}-title`
    const keepId = `${ids}-keep`
    const ballotIdLabelId = `${ids}-ballot-id`
    const qrCaptionId = `${ids}-qr`
    // With no tracker but a click handler (the portal's demo mode) the identifier
    // is an action, not a link: an anchor without `href` is out of the tab order.
    const ballotIdElement = ballotIdHref === undefined && onBallotIdClick ? "button" : "a"
    // "City council · 1 November 2026, 14:32"; the time joins once it is known.
    const meta = [electionName, castAt].filter((part) => Boolean(part)).join(" · ")

    const ballotIdLink = (
        <BallotIdLink
            component={ballotIdElement}
            data-testid="ballot-id"
            className={`ballot-id-value${shortBallotId === undefined ? " ballot-id-value-full" : ""}`}
            href={ballotIdHref}
            target={ballotIdHref === undefined ? undefined : (ballotIdTarget ?? undefined)}
            onClick={onBallotIdClick}
            sx={
                ballotIdElement === "button"
                    ? {
                          font: "inherit",
                          textAlign: "left",
                          background: "none",
                          border: 0,
                          p: 0,
                          cursor: "pointer",
                          minWidth: 0,
                      }
                    : {minWidth: 0}
            }
        >
            {shortBallotId === undefined ? (
                <FullBallotId className="ballot-id-full">{ballotId}</FullBallotId>
            ) : (
                <ShortBallotId className="ballot-id-short">{shortBallotId}</ShortBallotId>
            )}
        </BallotIdLink>
    )

    const ticket = (
        <Ticket component="aside" className="confirmation-ticket" aria-labelledby={`${ids}-rc`}>
            <TicketHead className="confirmation-ticket-head">
                <Box
                    id={`${ids}-rc`}
                    className="confirmation-ticket-eyebrow"
                    sx={{
                        fontSize: "12px",
                        lineHeight: "16px",
                        fontWeight: 700,
                        letterSpacing: "0.08em",
                        textTransform: "uppercase",
                        color: "rgba(255, 255, 255, 0.7)",
                    }}
                >
                    {t("confirmationScreen.receipt")}
                </Box>
                {eventName ? (
                    <Box
                        className="confirmation-ticket-event"
                        sx={{mt: "4px", fontSize: "18px", lineHeight: "24px", fontWeight: 700}}
                    >
                        {eventName}
                    </Box>
                ) : null}
                <Box
                    className="confirmation-ticket-meta"
                    sx={{
                        mt: "2px",
                        minHeight: "20px",
                        fontSize: "14px",
                        lineHeight: "20px",
                        color: "rgba(255, 255, 255, 0.8)",
                        fontVariantNumeric: "tabular-nums",
                    }}
                >
                    {meta}
                </Box>
            </TicketHead>
            <TicketBody className="confirmation-ticket-body">
                <BallotIdContainer className="ballot-id-container">
                    <Box
                        className="ballot-id-heading"
                        sx={{display: "flex", alignItems: "center", gap: "2px", minHeight: "26px"}}
                    >
                        <Typography
                            id={ballotIdLabelId}
                            className="ballot-id-label"
                            variant="h5"
                            component="h2"
                            sx={{
                                m: 0,
                                fontSize: "12px",
                                lineHeight: "16px",
                                fontWeight: 700,
                                letterSpacing: "0.06em",
                                textTransform: "uppercase",
                                color: COLORS.label,
                            }}
                        >
                            {t("confirmationScreen.ballotId")}
                        </Typography>
                        {onBallotIdHelp === undefined ? null : (
                            <IconButton
                                buttonClassName="ballot-id-help-button"
                                icon={faCircleQuestion}
                                sx={{
                                    "width": 26,
                                    "height": 26,
                                    "p": 0,
                                    "fontSize": "unset",
                                    "lineHeight": "unset",
                                    "color": COLORS.label,
                                    "&:hover": {backgroundColor: "rgba(0, 0, 0, 0.05)"},
                                }}
                                fontSize="0.9375rem"
                                onClick={onBallotIdHelp}
                                ariaLabel={t("a11y.helpAbout", {
                                    topic: t("confirmationScreen.ballotId"),
                                })}
                            />
                        )}
                    </Box>
                    <BallotIdLine className="ballot-id-line">
                        <BallotIdBorder
                            className="ballot-id-border"
                            sx={
                                shortBallotId === undefined
                                    ? {flex: "1 1 auto"}
                                    : {p: "4px 12px 4px 6px"}
                            }
                        >
                            {ballotIdStatus ?? (
                                <DecorativeIconBox className="ballot-id-status-icon">
                                    <Icon
                                        className="ballot-id-check-icon"
                                        icon={faCheck}
                                        style={{
                                            fontSize: "0.875rem",
                                            lineHeight: "unset",
                                            paddingBottom: "2px",
                                        }}
                                        color={theme.palette.customGrey.contrastText}
                                    />
                                </DecorativeIconBox>
                            )}
                            <BallotIdText className="ballot-id-text">{ballotIdLink}</BallotIdText>
                            {ballotIdCopyLabels === undefined ? null : (
                                <BallotHashCopyButton
                                    hash={ballotIdCopyValue ?? ballotId}
                                    copyLabels={ballotIdCopyLabels}
                                    sx={ballotIdButtonSx}
                                    copiedColor={ballotIdCopiedColor}
                                />
                            )}
                        </BallotIdBorder>
                        {ballotIdTag}
                    </BallotIdLine>
                </BallotIdContainer>
                {ballotIdDetails === undefined || ballotIdDetails === null ? null : (
                    <Box className="confirmation-ticket-details" sx={{mt: "16px"}}>
                        {ballotIdDetails}
                    </Box>
                )}
            </TicketBody>
            {qrValue === undefined ? null : (
                <>
                    <Perforation className="confirmation-ticket-perforation" aria-hidden="true" />
                    <QRFigure className="qr-container">
                        <QRCode ariaLabelledby={qrCaptionId} value={qrValue} size={112} />
                        <Box
                            component="figcaption"
                            id={qrCaptionId}
                            className="qr-caption"
                            sx={{fontSize: "13px", lineHeight: "19px", color: COLORS.body}}
                        >
                            {t("confirmationScreen.qrCaption")}
                        </Box>
                    </QRFigure>
                </>
            )}
        </Ticket>
    )

    return (
        <PageLimit maxWidth="lg" className="confirmation-screen screen">
            {steps === undefined ? null : (
                <Box className="stepper-box" marginTop="24px" sx={{[PHONE]: {marginTop: "16px"}}}>
                    {steps}
                </Box>
            )}
            <Columns
                className={`confirmation-columns${isFullyAcclaimed ? " confirmation-columns-single" : ""}`}
            >
                <Outcome
                    component="section"
                    className="confirmation-outcome"
                    aria-labelledby={titleId}
                >
                    <OutcomeIcon className="confirmation-outcome-icon" aria-hidden="true">
                        <BallotBoxMark />
                    </OutcomeIcon>
                    <Box className="confirmation-outcome-text" sx={{minWidth: 0}}>
                        <Typography
                            id={titleId}
                            className="screen-title"
                            variant="h4"
                            component="h1"
                            sx={{
                                m: 0,
                                fontSize: "32px",
                                lineHeight: "38px",
                                fontWeight: 700,
                                letterSpacing: "-0.01em",
                                [PHONE]: {fontSize: "24px", lineHeight: "30px"},
                            }}
                        >
                            <span className="screen-title-text">
                                {t(
                                    isFullyAcclaimed
                                        ? "confirmationScreen.acclamation.title"
                                        : "confirmationScreen.title"
                                )}
                            </span>
                        </Typography>
                        {isFullyAcclaimed && !meta ? null : (
                            <Meta className="confirmation-meta">{meta}</Meta>
                        )}
                        {!isFullyAcclaimed ? null : (
                            <Typography
                                className="screen-description"
                                variant="body2"
                                component="div"
                                sx={{mt: "8px", color: COLORS.body}}
                            >
                                {stringToHtml(t("confirmationScreen.acclamation.description"))}
                            </Typography>
                        )}
                        {!isBlankBallot ? null : (
                            <Typography
                                className="blank-ballot-description"
                                variant="body2"
                                component="div"
                                sx={{mt: "8px", color: COLORS.body}}
                            >
                                {stringToHtml(t("confirmationScreen.blankBallot.description"))}
                            </Typography>
                        )}
                    </Box>
                </Outcome>
                {/* A fully acclaimed election casts no ballot, so it has no
                    ticket, identifier, tracker link or QR code to show. */}
                {isFullyAcclaimed ? null : ticket}
                {isFullyAcclaimed ? (
                    <Section className="confirmation-keep">{actions}</Section>
                ) : (
                    <Section
                        component="section"
                        className="confirmation-keep"
                        aria-labelledby={keepId}
                    >
                        <Typography
                            id={keepId}
                            className="confirmation-keep-title"
                            variant="h5"
                            component="h2"
                            sx={{m: 0, fontSize: "18px", lineHeight: "24px", fontWeight: 700}}
                        >
                            {t("confirmationScreen.keepTitle")}
                        </Typography>
                        <Lead component="p" className="confirmation-keep-lead">
                            {t("confirmationScreen.keepLead")}
                        </Lead>
                        {actions}
                    </Section>
                )}
                {updates === undefined || updates === null || updates === false ? null : (
                    <Updates className="confirmation-updates">{updates}</Updates>
                )}
            </Columns>
            {/* The host's dialogs render into a portal, so they sit outside the
                ticket and survive when an acclaimed election hides it. */}
            {children}
        </PageLimit>
    )
}
