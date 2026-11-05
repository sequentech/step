// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The parts the Approvals screens share: cards, tags, option cards and the
// two-way buttons of the example and the rule editor.
import {styled} from "@mui/material/styles"

const BORDER = "#E3E7EF"
const MUTED = "#5B6272"

/** The tinted page the matrix sits on. */
export const Page = styled("div")(({theme}) => ({
    background: theme.palette.lightBackground,
    borderRadius: "16px",
    padding: "32px",
    marginTop: "24px",
    [theme.breakpoints.down("md")]: {
        padding: "16px",
    },
}))

export const Card = styled("section")(({theme}) => ({
    background: theme.palette.white,
    border: `1px solid ${BORDER}`,
    borderRadius: "12px",
    padding: "24px",
    boxShadow: "0 1px 2px rgba(15, 5, 76, 0.06)",
    [theme.breakpoints.down("sm")]: {
        padding: "16px",
    },
}))

export const CardTitle = styled("h3")({
    display: "flex",
    alignItems: "center",
    gap: "12px",
    margin: "0 0 16px",
    fontSize: "18px",
    fontWeight: 600,
    lineHeight: "24px",
})

/** The square icon in front of a card's title. */
export const CardIcon = styled("span")(({theme}) => ({
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    width: "36px",
    height: "36px",
    borderRadius: "8px",
    background: "#ECEEFB",
    color: theme.palette.brandColor,
    flexShrink: 0,
}))

export const Help = styled("p")({
    margin: "0 0 16px",
    color: MUTED,
    fontSize: "14px",
    lineHeight: "22px",
})

export const Muted = styled("span")({
    color: MUTED,
    fontSize: "14px",
    lineHeight: "20px",
})

/** A small capital heading, as over each group of the example. */
export const Overline = styled("div")({
    margin: "16px 0 8px",
    color: "#3D4353",
    fontSize: "12px",
    fontWeight: 600,
    letterSpacing: "0.04em",
    textTransform: "uppercase",
})

/** A condition or a fact, such as "Exactly 1 detail differs". */
export const Tag = styled("span")({
    display: "inline-flex",
    alignItems: "center",
    gap: "4px",
    padding: "3px 10px",
    borderRadius: "6px",
    border: "1px solid #D9DEE8",
    background: "#F1F3F8",
    color: "#2D3348",
    fontSize: "13px",
    fontWeight: 500,
    lineHeight: "20px",
    whiteSpace: "nowrap",
})

/** A tag that points at something, such as "Applies to your example". */
export const AccentTag = styled(Tag)(({theme}) => ({
    background: "#ECEEFB",
    borderColor: "#C9CEF2",
    color: theme.palette.blue.main,
}))

export const TagRow = styled("div")({
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "8px",
})

/** One of two or more choices side by side; `aria-pressed` marks the chosen one. */
export const Segment = styled("button")(({theme}) => ({
    "padding": "6px 12px",
    "borderRadius": "6px",
    "border": `1px solid ${BORDER}`,
    "background": theme.palette.white,
    "color": "#191D23",
    "fontFamily": "inherit",
    "fontSize": "14px",
    "fontWeight": 600,
    "lineHeight": "20px",
    "cursor": "pointer",
    "&[aria-pressed='true']": {
        background: theme.palette.brandColor,
        borderColor: theme.palette.brandColor,
        color: theme.palette.white,
    },
    "&:focus-visible": {
        outline: `2px solid ${theme.palette.brandSuccess}`,
        outlineOffset: "2px",
    },
    "&:disabled": {
        opacity: 0.5,
        cursor: "default",
    },
}))

export const SegmentGroup = styled("div")({
    display: "flex",
    flexWrap: "wrap",
    gap: "6px",
})

/** A choice shown as a card with a radio button; `data-selected` marks the chosen one. */
export const OptionCard = styled("label")(({theme}) => ({
    "display": "flex",
    "alignItems": "flex-start",
    "gap": "12px",
    "padding": "14px 16px",
    "border": `1px solid ${BORDER}`,
    "borderRadius": "10px",
    "background": theme.palette.white,
    "cursor": "pointer",
    "&[data-selected='true']": {
        borderColor: theme.palette.brandColor,
        boxShadow: `inset 0 0 0 1px ${theme.palette.brandColor}`,
        background: "#F4F3FB",
    },
    "&[data-disabled='true']": {
        background: "#F5F6FA",
        cursor: "default",
    },
    "&:focus-within": {
        outline: `2px solid ${theme.palette.brandSuccess}`,
        outlineOffset: "2px",
    },
}))

export const OptionTitle = styled("div")({
    fontSize: "15px",
    fontWeight: 600,
    lineHeight: "22px",
})

/** The round badge with a rule's number or a step's number. */
export const NumberBadge = styled("span")(({theme}) => ({
    "display": "inline-flex",
    "alignItems": "center",
    "justifyContent": "center",
    "width": "32px",
    "height": "32px",
    "borderRadius": "50%",
    "background": theme.palette.brandColor,
    "color": theme.palette.white,
    "fontSize": "14px",
    "fontWeight": 600,
    "flexShrink": 0,
    "&[data-pending='true']": {
        background: theme.palette.white,
        color: theme.palette.brandColor,
        border: `1.5px solid ${theme.palette.brandColor}`,
    },
    "&[data-muted='true']": {
        background: "#F1F3F8",
        color: MUTED,
    },
}))

/** The applicant's or a voter's initials. */
export const Avatar = styled("span")({
    "display": "inline-flex",
    "alignItems": "center",
    "justifyContent": "center",
    "width": "36px",
    "height": "36px",
    "borderRadius": "50%",
    "background": "#E3F4EC",
    "color": "#0B5A3C",
    "fontSize": "13px",
    "fontWeight": 700,
    "flexShrink": 0,
    "&[data-size='large']": {
        width: "56px",
        height: "56px",
        fontSize: "20px",
        background: "#FBDDE3",
        color: "#9B1C3A",
    },
})

/** The colored box of a message: `data-tone` is "warning", "info", "error" or "success". */
export const Notice = styled("div")({
    "display": "flex",
    "gap": "12px",
    "padding": "16px",
    "borderRadius": "8px",
    "fontSize": "14px",
    "lineHeight": "22px",
    "&[data-tone='warning']": {background: "#FFF3E2", color: "#3D2A00"},
    "&[data-tone='review']": {
        background: "#FFF6D8",
        border: "1px solid #F0D58C",
        color: "#3D2A00",
    },
    "&[data-tone='info']": {background: "#ECEEFB", border: "1px solid #C9CEF2", color: "#1D2470"},
    "&[data-tone='error']": {
        background: "#FDECEC",
        border: "1px solid #F5C2C2",
        color: "#991B1B",
    },
    "&[data-tone='success']": {
        background: "#ECFDF5",
        border: "1px solid #B7E4CC",
        color: "#064E3B",
    },
    "&[data-tone='neutral']": {
        background: "#F7F8FC",
        border: `1px solid ${BORDER}`,
        color: "#191D23",
    },
})

export const NoticeTitle = styled("div")({
    fontSize: "15px",
    fontWeight: 600,
    marginBottom: "4px",
})
