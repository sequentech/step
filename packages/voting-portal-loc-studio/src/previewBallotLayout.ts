// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const frame = ".loc-studio-preview-frame"

const candidateLists = [
    `${frame} .candidates-singles-container`,
    `${frame} .candidates-top-blank-invalid`,
    `${frame} .candidates-bottom-blank-invalid`,
    `${frame} .candidates-review-blank`,
    `${frame} .candidates-list-children`,
].join(",\n")

export const previewBallotLayoutStyles = {
    [candidateLists]: {
        display: "flex",
        flexDirection: "column",
        columnCount: "auto !important",
        columns: "auto !important",
        gap: "12px",
        margin: "12px 0 0",
        padding: 0,
        listStyle: "none",
    },
    [`${frame} .candidates-lists-container`]: {
        display: "flex",
        flexDirection: "column",
        gap: "12px",
    },
    [`${frame} .candidates-list`]: {
        width: "100% !important",
        maxWidth: "100%",
        boxSizing: "border-box",
    },
    [`${frame} li.candidate-item`]: {
        height: "auto !important",
        minHeight: "64px",
        maxHeight: "none !important",
        width: "100%",
        boxSizing: "border-box",
        flexGrow: "0 !important",
        alignItems: "flex-start",
        padding: "10px 8px !important",
        margin: "0 !important",
        breakInside: "auto !important",
        pageBreakInside: "auto !important",
        overflow: "visible",
    },
    [`${frame} li.candidate-item > .image-box`]: {
        flexShrink: 0,
        alignSelf: "center",
    },
    [`${frame} li.candidate-item > div:nth-of-type(2)`]: {
        minWidth: 0,
        flex: "1 1 auto",
        paddingTop: "2px",
    },
    [`${frame} li.candidate-item > .MuiCheckbox-root`]: {
        flexShrink: 0,
        alignSelf: "center",
        marginTop: "10px",
    },
    [`${frame} li.candidate-item > .candidate-link`]: {
        flexShrink: 0,
        alignSelf: "center",
    },
    [`${frame} .candidate-title`]: {
        lineHeight: 1.3,
        marginTop: "0 !important",
        marginBottom: "2px !important",
        overflowWrap: "anywhere",
    },
    [`${frame} .candidate-description`]: {
        lineHeight: 1.35,
        marginTop: "0 !important",
        marginBottom: "0 !important",
        overflowWrap: "anywhere",
    },
    [`${frame} .candidate-writein-textfield`]: {
        width: "100%",
        maxWidth: "100%",
        marginTop: "6px",
    },
} as const
