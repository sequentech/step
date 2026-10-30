// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot tracker's search (design board TrackerSearch): a card that asks
// for the Tracking ID, with the formats it takes and the ballots that share
// a short ID to choose from, and under it what the tracker shows.

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, ButtonBase, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {translateHtml} from "@sequentech/ui-core"
import {typedBallotId} from "../../services/vitrine/ballotId"
import {rejectedBallotId} from "../../services/vitrine/ballotTracker"
import {CHECK_COLORS} from "../Vitrine/BallotStatusIcon"
import {MONO, PALETTE, PHONE} from "./trackerStyle"

const FIELD_ID = "tracker-search-id"
const TITLE_ID = "tracker-search-title"
const HELP_ID = "ballot-id-help"
const ERROR_ID = "ballot-id-error"
const SHORT_ID_EXAMPLE = "30ff d314"
const BORDER = "#CBD5E1"
const BORDER_HOVER = "#94A3B8"

const CHECK_PATHS = ["M1.5 12.75l4.5 4.5L16.25 7", "M10.75 15.75l1.5 1.5L22.5 7"]

/** The double check of the tracker, in the navy and the blue of the check
 * mark of a verified ballot. */
const DoubleCheck: React.FC<{size: number; first: string; second: string}> = ({
    size,
    first,
    second,
}) => (
    <svg
        className="tracker-search-check"
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
        style={{overflow: "visible"}}
    >
        <path className="tracker-check-path-first" d={CHECK_PATHS[0]} stroke={first} />
        <path className="tracker-check-path-second" d={CHECK_PATHS[1]} stroke={second} />
    </svg>
)

const FIELD_BOX = {
    "display": "flex",
    "alignItems": "center",
    "gap": "10px",
    "padding": "6px 6px 6px 16px",
    "border": `2px solid ${BORDER}`,
    "borderRadius": "14px",
    "backgroundColor": "#fff",
    "transition": "border-color .15s, box-shadow .15s",
    "&:hover": {borderColor: BORDER_HOVER},
    "&:focus-within": {
        borderColor: PALETTE.navy,
        boxShadow: "0 0 0 4px rgba(15, 5, 76, 0.1)",
    },
} as const

const FIELD_BOX_INVALID = {
    "borderColor": PALETTE.redBorder,
    "&:hover, &:focus-within": {
        borderColor: PALETTE.redBorder,
        boxShadow: "0 0 0 4px rgba(220, 38, 38, 0.1)",
    },
} as const

const FindButton = styled(ButtonBase)`
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 52px;
    padding: 6px 20px;
    border: 1px solid ${PALETTE.navy};
    border-radius: 10px;
    background: ${PALETTE.navy};
    color: #fff;
    font-size: 16px;
    font-weight: 500;
    line-height: 1.75;
    white-space: nowrap;

    &.Mui-disabled {
        background: ${PALETTE.border};
        border-color: ${PALETTE.border};
        color: #757575;
    }

    ${PHONE} {
        padding: 6px 14px;
    }
`

const Chip: React.FC<{textKey: string}> = ({textKey}) => {
    const {t} = useTranslation()
    return (
        <Box
            component="span"
            className="ballot-id-chip"
            sx={{
                "padding": "4px 10px",
                "borderRadius": "999px",
                "backgroundColor": "#fff",
                "border": `1px solid ${PALETTE.border}`,
                "fontSize": 12,
                "lineHeight": "16px",
                "color": PALETTE.muted,
                "& b": {fontWeight: 600, color: PALETTE.text},
            }}
        >
            {translateHtml(t, textKey)}
        </Box>
    )
}

const FEATURES = [
    {key: "cast", tone: CHECK_COLORS.BLUE},
    {key: "audited", tone: PALETTE.green},
    {key: "secret", tone: PALETTE.navy},
] as const

type TFeature = (typeof FEATURES)[number]["key"]

const FEATURE_ICONS: Record<TFeature, (color: string) => React.ReactNode> = {
    cast: (color) => <DoubleCheck size={20} first={color} second={color} />,
    audited: (color) => (
        <svg
            className="tracker-feature-audited-icon"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke={color}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            <rect
                className="tracker-feature-audited-icon-body"
                x="4"
                y="3"
                width="16"
                height="18"
                rx="2"
            />
            <path className="tracker-feature-audited-icon-lines" d="M8 8h8M8 12h8M8 16h5" />
        </svg>
    ),
    secret: (color) => (
        <svg
            className="tracker-feature-secret-icon"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke={color}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            <rect
                className="tracker-feature-secret-icon-body"
                x="5"
                y="11"
                width="14"
                height="10"
                rx="2"
            />
            <path className="tracker-feature-secret-icon-shackle" d="M8 11V8a4 4 0 0 1 8 0v3" />
        </svg>
    ),
}

export interface ITrackerSearchProps {
    /** What the field shows: the ID in groups of four. */
    value: string
    onChange: (value: string) => void
    onSubmit: () => void
    /** Whether the field holds an ID to find a ballot by. */
    canSubmit: boolean
    /** The ballots that share the short ID, to choose from. */
    children?: React.ReactNode
}

export const TrackerSearch: React.FC<ITrackerSearchProps> = ({
    value,
    onChange,
    onSubmit,
    canSubmit,
    children,
}) => {
    const {t} = useTranslation()
    const [rejected, setRejected] = useState(false)

    return (
        <>
            <Box
                component="section"
                className="tracker-search"
                aria-labelledby={TITLE_ID}
                sx={{
                    marginTop: "8px",
                    padding: "48px 32px 40px",
                    border: `1px solid ${PALETTE.border}`,
                    borderRadius: "20px",
                    backgroundColor: PALETTE.tint,
                    [PHONE]: {padding: "32px 16px 24px", borderRadius: "16px"},
                }}
            >
                <Box
                    className="tracker-search-content"
                    sx={{
                        maxWidth: 680,
                        margin: "0 auto",
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        textAlign: "center",
                    }}
                >
                    <Box
                        component="span"
                        className="tracker-search-seal"
                        aria-hidden="true"
                        sx={{
                            width: 64,
                            height: 64,
                            borderRadius: "50%",
                            backgroundColor: "#fff",
                            border: `1px solid ${PALETTE.border}`,
                            boxShadow: "0 2px 8px rgba(15, 5, 76, 0.06)",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                        }}
                    >
                        <DoubleCheck size={30} first={PALETTE.navy} second={CHECK_COLORS.BLUE} />
                    </Box>
                    <Typography
                        component="h2"
                        id={TITLE_ID}
                        className="tracker-search-title"
                        sx={{
                            margin: "18px 0 8px",
                            textAlign: "center",
                            fontSize: 28,
                            lineHeight: 1.25,
                            fontWeight: 700,
                            color: PALETTE.navy,
                            [PHONE]: {fontSize: 22},
                        }}
                    >
                        {t("ballotLocator.search.title")}
                    </Typography>
                    <Typography
                        className="tracker-search-intro"
                        sx={{
                            margin: "0 0 28px",
                            maxWidth: 520,
                            textAlign: "center",
                            fontSize: 16,
                            lineHeight: 1.5,
                            color: PALETTE.muted,
                            [PHONE]: {fontSize: 15, marginBottom: "20px"},
                        }}
                    >
                        {t("ballotLocator.search.intro")}
                    </Typography>

                    <Box className="tracker-search-field" sx={{width: "100%", textAlign: "left"}}>
                        <Box
                            component="label"
                            htmlFor={FIELD_ID}
                            className="tracker-search-label"
                            sx={{
                                display: "block",
                                margin: "0 0 6px 4px",
                                fontSize: 13,
                                fontWeight: 600,
                                color: PALETTE.muted,
                            }}
                        >
                            {t("a11y.ballotIdLabel")}
                        </Box>
                        <Box
                            className="ballot-id-field"
                            sx={rejected ? {...FIELD_BOX, ...FIELD_BOX_INVALID} : FIELD_BOX}
                        >
                            <svg
                                className="tracker-search-icon"
                                width="22"
                                height="22"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke={PALETTE.quiet}
                                strokeWidth="2"
                                strokeLinecap="round"
                                aria-hidden="true"
                                focusable="false"
                                style={{flex: "none"}}
                            >
                                <circle
                                    className="tracker-search-icon-lens"
                                    cx="11"
                                    cy="11"
                                    r="6.5"
                                />
                                <path className="tracker-search-icon-handle" d="M16 16l4.5 4.5" />
                            </svg>
                            <Box
                                component="input"
                                id={FIELD_ID}
                                className="ballot-id-input"
                                value={value}
                                placeholder={SHORT_ID_EXAMPLE}
                                autoComplete="off"
                                spellCheck={false}
                                aria-invalid={rejected}
                                aria-describedby={rejected ? ERROR_ID : HELP_ID}
                                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                                    setRejected(rejectedBallotId(event.target.value))
                                    onChange(typedBallotId(event.target.value))
                                }}
                                onKeyDown={(event: React.KeyboardEvent<HTMLInputElement>) => {
                                    if (event.key === "Enter") onSubmit()
                                }}
                                sx={{
                                    "flex": 1,
                                    "minWidth": 0,
                                    "height": 52,
                                    "padding": 0,
                                    "border": 0,
                                    "outline": "none",
                                    "background": "transparent",
                                    "font": `600 18px ${MONO}`,
                                    "letterSpacing": ".04em",
                                    "color": PALETTE.navy,
                                    "&::placeholder": {color: PALETTE.faint, fontWeight: 400},
                                }}
                            />
                            <FindButton
                                className="locate-ballot-button"
                                disabled={!canSubmit}
                                onClick={onSubmit}
                            >
                                <span className="locate-ballot-label">
                                    {t("ballotLocator.search.find")}
                                </span>
                                <svg
                                    className="tracker-find-icon"
                                    width="16"
                                    height="16"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    strokeWidth="2.6"
                                    strokeLinecap="round"
                                    strokeLinejoin="round"
                                    aria-hidden="true"
                                    focusable="false"
                                >
                                    <path
                                        className="tracker-find-icon-arrow"
                                        d="M5 12h14M13 6l6 6-6 6"
                                    />
                                </svg>
                            </FindButton>
                        </Box>
                        {/* The live region stays mounted and only its content changes: a
                            region inserted at the same moment as its text is not reliably read. */}
                        <Box
                            className="ballot-id-error"
                            id={ERROR_ID}
                            role="alert"
                            sx={{
                                display: "flex",
                                alignItems: "center",
                                gap: "6px",
                                margin: rejected ? "10px 0 0 4px" : 0,
                                fontSize: 13,
                                lineHeight: "18px",
                                color: PALETTE.redInk,
                            }}
                        >
                            {rejected ? (
                                <>
                                    <svg
                                        className="tracker-invalid-icon"
                                        width="14"
                                        height="14"
                                        viewBox="0 0 24 24"
                                        aria-hidden="true"
                                        focusable="false"
                                        style={{flex: "none"}}
                                    >
                                        <circle
                                            className="tracker-invalid-icon-disc"
                                            cx="12"
                                            cy="12"
                                            r="11"
                                            fill="currentColor"
                                        />
                                        <path
                                            className="tracker-invalid-icon-mark"
                                            d="M12 7v6M12 16.5v.1"
                                            stroke="#fff"
                                            strokeWidth="2.6"
                                            strokeLinecap="round"
                                        />
                                    </svg>
                                    {t("ballotLocator.search.invalid")}
                                </>
                            ) : null}
                        </Box>
                        {rejected ? null : (
                            <Box
                                className="ballot-id-help"
                                id={HELP_ID}
                                sx={{
                                    display: "flex",
                                    flexWrap: "wrap",
                                    gap: "8px",
                                    margin: "12px 0 0 4px",
                                }}
                            >
                                <Chip textKey="ballotLocator.search.chips.short" />
                                <Chip textKey="ballotLocator.search.chips.full" />
                                <Chip textKey="ballotLocator.search.chips.spaces" />
                            </Box>
                        )}
                    </Box>
                    {children}
                </Box>
            </Box>

            <Box
                component="ul"
                className="tracker-features"
                sx={{
                    listStyle: "none",
                    margin: "24px 0 0",
                    padding: 0,
                    display: "grid",
                    gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
                    gap: "16px",
                    [PHONE]: {gridTemplateColumns: "1fr"},
                }}
            >
                {FEATURES.map(({key, tone}) => (
                    <Box
                        component="li"
                        key={key}
                        className={`tracker-feature tracker-feature-${key}`}
                        sx={{
                            padding: "20px",
                            border: `1px solid ${PALETTE.border}`,
                            borderRadius: "16px",
                            backgroundColor: "#fff",
                        }}
                    >
                        <Box
                            className="tracker-feature-icon-box"
                            component="span"
                            aria-hidden="true"
                            sx={{
                                width: 40,
                                height: 40,
                                borderRadius: "12px",
                                backgroundColor: PALETTE.tint,
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                            }}
                        >
                            {FEATURE_ICONS[key](tone)}
                        </Box>
                        <Box
                            className="tracker-feature-title"
                            sx={{
                                marginTop: "14px",
                                fontSize: 16,
                                lineHeight: "22px",
                                fontWeight: 700,
                            }}
                        >
                            {t(`ballotLocator.search.features.${key}.title`)}
                        </Box>
                        <Typography
                            className="tracker-feature-text"
                            sx={{
                                margin: "4px 0 0",
                                fontSize: 14,
                                lineHeight: "20px",
                                color: PALETTE.muted,
                            }}
                        >
                            {t(`ballotLocator.search.features.${key}.text`)}
                        </Typography>
                    </Box>
                ))}
            </Box>
        </>
    )
}
