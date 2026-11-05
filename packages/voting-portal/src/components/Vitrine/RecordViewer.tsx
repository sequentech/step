// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A record's text in a card (design board RecordViewer): a line of preview, or
// all of it laid out with line numbers and colours, to copy or to save.

import React, {useId, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, ButtonBase} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useCopyFeedback} from "../../hooks/useCopyFeedback"
import {saveTextFile} from "../../services/vitrine/file"
import {
    byteSizeOf,
    ERecordFormat,
    ETokenKind,
    RECORD_MEDIA_TYPES,
    readRecordText,
    recordLines,
} from "../../services/vitrine/record"
import {CHECK_COLORS} from "./BallotStatusIcon"

const PALETTE = {
    border: "#E7EAEE",
    tint: "#F7F9FE",
    text: "#191D23",
    navy: "#0F054C",
    quiet: "#64748B",
    faint: "#94A3B8",
    badge: "#EEF1FB",
    hover: "#F1F5F9",
    green: "#047857",
} as const

const MONO = "ui-monospace,SFMono-Regular,Menlo,Consolas,monospace"
const META_SEPARATOR = " · "
/** Only the start of a record fits the line of preview. */
const PREVIEW_LENGTH = 400

const TOKEN_STYLES: Record<ETokenKind, {color: string; fontWeight: number}> = {
    [ETokenKind.KEY]: {color: PALETTE.navy, fontWeight: 600},
    [ETokenKind.STRING]: {color: PALETTE.green, fontWeight: 400},
    [ETokenKind.NUMBER]: {color: CHECK_COLORS.AMBER, fontWeight: 400},
    [ETokenKind.LITERAL]: {color: CHECK_COLORS.BLUE, fontWeight: 400},
    [ETokenKind.PUNCTUATION]: {color: PALETTE.quiet, fontWeight: 400},
    [ETokenKind.TEXT]: {color: PALETTE.text, fontWeight: 400},
}

const ICON_PROPS = {
    "width": 18,
    "height": 18,
    "viewBox": "0 0 24 24",
    "fill": "none",
    "strokeLinecap": "round",
    "strokeLinejoin": "round",
    "focusable": false,
    "aria-hidden": true,
} as const

const IconAction = styled(ButtonBase)`
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 50%;
    color: ${PALETTE.text};
    transition: background 0.15s;

    &:hover {
        background: ${PALETTE.hover};
    }
`

export interface IRecordViewerProps {
    title: string
    /** What the record is, ahead of its format and size. */
    subtitle?: string
    /** The file the record is saved as. */
    filename: string
    content: string
    className?: string
}

export const RecordViewer: React.FC<IRecordViewerProps> = ({
    title,
    subtitle,
    filename,
    content,
    className,
}) => {
    const {t, i18n} = useTranslation()
    const [open, setOpen] = useState(false)
    const {copied, copy} = useCopyFeedback()
    const titleId = useId()
    const record = useMemo(() => readRecordText(content), [content])
    const lines = useMemo(() => (open ? recordLines(record) : []), [open, record])
    const size = byteSizeOf(record.bytes, i18n.resolvedLanguage || i18n.language)
    const meta = [
        subtitle,
        t(record.format === ERecordFormat.JSON ? "vitrine.record.json" : "vitrine.record.text"),
        record.count
            ? t(`vitrine.record.${record.count.kind}`, {count: record.count.count})
            : undefined,
        t(`vitrine.record.size.${size.unit}`, {value: size.value}),
    ]
        .filter((part): part is string => Boolean(part))
        .join(META_SEPARATOR)

    return (
        <Box
            component="section"
            className={`record-viewer ${className ?? ""}`.trim()}
            aria-label={title}
            sx={{
                boxSizing: "border-box",
                width: "100%",
                minWidth: 0,
                border: `1px solid ${PALETTE.border}`,
                borderRadius: "8px",
                backgroundColor: "#fff",
                overflow: "hidden",
                color: PALETTE.text,
            }}
        >
            <Box
                className="record-viewer-bar"
                sx={{display: "flex", alignItems: "center", gap: "2px", padding: "4px 6px 4px 0"}}
            >
                <ButtonBase
                    className="record-viewer-toggle"
                    aria-expanded={open}
                    onClick={() => setOpen(!open)}
                    sx={{
                        "flex": 1,
                        "minWidth": 0,
                        "justifyContent": "flex-start",
                        "gap": "12px",
                        "minHeight": 52,
                        "padding": "6px 8px 6px 10px",
                        "borderRadius": "6px",
                        "textAlign": "left",
                        "&:hover .record-viewer-chevron": {backgroundColor: PALETTE.hover},
                    }}
                >
                    <Box
                        component="span"
                        className="record-viewer-chevron"
                        sx={{
                            width: 28,
                            height: 28,
                            flex: "none",
                            borderRadius: "50%",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            transition: "background 0.15s",
                        }}
                    >
                        <svg
                            className="record-viewer-chevron"
                            {...ICON_PROPS}
                            width={16}
                            height={16}
                            strokeWidth={2.4}
                            stroke="#475569"
                            style={{
                                transform: `rotate(${open ? 90 : 0}deg)`,
                                transition: "transform 0.2s",
                            }}
                        >
                            <path className="record-viewer-chevron-path" d="M9 6l6 6-6 6" />
                        </svg>
                    </Box>
                    <Box
                        component="span"
                        className="record-viewer-badge"
                        aria-hidden="true"
                        sx={{
                            width: 36,
                            height: 36,
                            flex: "none",
                            borderRadius: "8px",
                            backgroundColor: PALETTE.badge,
                            color: PALETTE.navy,
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            font: `700 12px/1 ${MONO}`,
                        }}
                    >
                        {"{ }"}
                    </Box>
                    <Box
                        className="record-viewer-text"
                        component="span"
                        sx={{display: "flex", flexDirection: "column", minWidth: 0}}
                    >
                        <Box
                            component="span"
                            id={titleId}
                            className="record-viewer-title"
                            sx={{fontSize: 15, lineHeight: "22px", fontWeight: 700}}
                        >
                            {title}
                        </Box>
                        <Box
                            component="span"
                            className="record-viewer-meta"
                            sx={{
                                fontSize: 13,
                                lineHeight: "18px",
                                color: PALETTE.quiet,
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                            }}
                        >
                            {meta}
                        </Box>
                    </Box>
                </ButtonBase>
                <IconAction
                    className="record-viewer-copy"
                    aria-label={
                        copied
                            ? t("vitrine.record.copied")
                            : t("vitrine.record.copy", {file: filename})
                    }
                    onClick={() => copy(record.pretty)}
                    sx={{color: copied ? PALETTE.green : PALETTE.text}}
                >
                    {copied ? (
                        <svg
                            className="record-viewer-copied-icon"
                            {...ICON_PROPS}
                            strokeWidth={2.6}
                            stroke="currentColor"
                        >
                            <path
                                className="record-viewer-copied-path"
                                d="M4.5 12.5l4.75 4.75L19.5 7"
                            />
                        </svg>
                    ) : (
                        <svg
                            className="record-viewer-copy-icon"
                            {...ICON_PROPS}
                            strokeWidth={2}
                            stroke="currentColor"
                        >
                            <rect
                                className="record-viewer-copy-back"
                                x="8"
                                y="8"
                                width="12"
                                height="12"
                                rx="2"
                            />
                            <path
                                className="record-viewer-copy-front"
                                d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"
                            />
                        </svg>
                    )}
                </IconAction>
                <IconAction
                    className="record-viewer-download"
                    aria-label={t("vitrine.record.download", {file: filename})}
                    onClick={() =>
                        saveTextFile(filename, record.pretty, RECORD_MEDIA_TYPES[record.format])
                    }
                >
                    <svg
                        className="record-viewer-download-icon"
                        {...ICON_PROPS}
                        strokeWidth={2}
                        stroke="currentColor"
                    >
                        <path
                            className="record-viewer-download-path"
                            d="M12 4v11M7 10l5 5 5-5M5 20h14"
                        />
                    </svg>
                </IconAction>
            </Box>
            {open ? (
                <Box
                    className="record-viewer-body"
                    role="group"
                    aria-labelledby={titleId}
                    tabIndex={0}
                    sx={{
                        borderTop: `1px solid ${PALETTE.border}`,
                        backgroundColor: PALETTE.tint,
                        maxHeight: 360,
                        overflow: "auto",
                        padding: "12px 0",
                        font: `400 13px/21px ${MONO}`,
                    }}
                >
                    {lines.map((tokens, n) => (
                        <Box key={n} className="record-viewer-line" sx={{display: "flex"}}>
                            <Box
                                component="span"
                                className="record-viewer-line-number"
                                aria-hidden="true"
                                sx={{
                                    width: 44,
                                    flex: "none",
                                    boxSizing: "border-box",
                                    paddingRight: "14px",
                                    textAlign: "right",
                                    color: PALETTE.faint,
                                    userSelect: "none",
                                }}
                            >
                                {n + 1}
                            </Box>
                            <Box
                                component="span"
                                className="record-viewer-code"
                                sx={{
                                    flex: 1,
                                    minWidth: 0,
                                    paddingRight: "16px",
                                    whiteSpace: "pre-wrap",
                                    overflowWrap: "anywhere",
                                }}
                            >
                                {tokens.map((token, i) => (
                                    <span
                                        key={i}
                                        className={`record-token record-token-${token.kind}`}
                                        style={TOKEN_STYLES[token.kind]}
                                    >
                                        {token.text}
                                    </span>
                                ))}
                            </Box>
                        </Box>
                    ))}
                </Box>
            ) : (
                <Box
                    className="record-viewer-preview"
                    aria-hidden="true"
                    sx={{
                        borderTop: `1px solid ${PALETTE.border}`,
                        backgroundColor: PALETTE.tint,
                        padding: "10px 16px",
                        font: `400 13px/20px ${MONO}`,
                        color: PALETTE.quiet,
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                    }}
                >
                    {record.compact.slice(0, PREVIEW_LENGTH)}
                </Box>
            )}
        </Box>
    )
}
