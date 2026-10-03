// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const TEXT_SIZE = "html[data-a11y-text-size]"
const SPACING = 'html[data-a11y-text-spacing="wide"]'
const MOTION = 'html[data-a11y-motion="reduced"]'
const CONTRAST = 'html[data-a11y-contrast="high"]'

/**
 * What each `data-a11y-*` attribute on `<html>` does.
 *
 * Text size scales the root font size, so it reaches everything sized in `rem`. Rows with a
 * fixed height are let grow, or larger and wider text would be clipped.
 *
 * High contrast only repaints: black text, black borders, and buttons inverted to white on
 * black. It never gives a background to an element that had none, because transparent
 * overlays (ripples, input outlines) sit on top of the text they decorate.
 */
export const accessibilityStyles = `
html[data-a11y-text-size="large"] {
    font-size: 125%;
}
html[data-a11y-text-size="larger"] {
    font-size: 150%;
}
${TEXT_SIZE} .candidate-item,
${SPACING} .candidate-item,
${TEXT_SIZE} .header-page-limit,
${SPACING} .header-page-limit {
    height: auto !important;
    min-height: 37px;
}
${TEXT_SIZE} .candidate-item,
${SPACING} .candidate-item {
    min-height: 64px;
}
${TEXT_SIZE} body *,
${SPACING} body * {
    overflow-wrap: anywhere !important;
    word-break: normal !important;
}
${TEXT_SIZE} body *:has(> .MuiButton-root),
${SPACING} body *:has(> .MuiButton-root) {
    flex-wrap: wrap !important;
    gap: 8px;
}
${TEXT_SIZE} body .MuiButton-root,
${SPACING} body .MuiButton-root {
    min-width: 0 !important;
    white-space: normal !important;
}

${SPACING} body,
${SPACING} body *:not(svg):not(svg *) {
    line-height: 1.5 !important;
    letter-spacing: 0.12em !important;
    word-spacing: 0.16em !important;
}
${SPACING} body p {
    margin-bottom: 2em !important;
}

${MOTION} *,
${MOTION} *::before,
${MOTION} *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
}

${CONTRAST} body {
    background-color: #fff !important;
    color: #000 !important;
}
${CONTRAST} body *:not(svg *) {
    color: #000 !important;
    border-color: #000 !important;
    text-shadow: none !important;
}
${CONTRAST} body a {
    text-decoration: underline !important;
}
${CONTRAST} body .MuiButton-root,
${CONTRAST} body .MuiButton-root:hover,
${CONTRAST} body .MuiButton-root:focus,
${CONTRAST} body .MuiButton-root:active,
${CONTRAST} body [aria-current="step"] .step-number {
    background: #000 !important;
    border: 2px solid #000 !important;
}
${CONTRAST} body .MuiButton-root,
${CONTRAST} body .MuiButton-root *:not(svg *),
${CONTRAST} body [aria-current="step"] .step-number {
    color: #fff !important;
}
${CONTRAST} body .MuiButton-root.Mui-disabled {
    background: #fff !important;
    border: 2px dashed #000 !important;
    opacity: 1 !important;
}
${CONTRAST} body .MuiButton-root.Mui-disabled,
${CONTRAST} body .MuiButton-root.Mui-disabled *:not(svg *) {
    color: #000 !important;
}
${CONTRAST} body .step-number {
    background: #fff !important;
    border: 2px solid #000 !important;
    opacity: 1 !important;
}
${CONTRAST} body .candidate-item,
${CONTRAST} body .MuiPaper-root,
${CONTRAST} body .MuiTooltip-tooltip {
    background-color: #fff !important;
    border: 2px solid #000 !important;
    box-shadow: none !important;
    opacity: 1 !important;
}
${CONTRAST} body .Mui-selected {
    outline: 2px solid #000 !important;
    outline-offset: -2px !important;
}
${CONTRAST} body .MuiLinearProgress-bar,
${CONTRAST} body .MuiSkeleton-root {
    background-color: #000 !important;
}
${CONTRAST} body :focus-visible {
    outline: 3px solid #000 !important;
    outline-offset: 2px !important;
    box-shadow: 0 0 0 6px #fff !important;
}
`
