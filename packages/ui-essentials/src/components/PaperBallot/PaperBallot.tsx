// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactNode, useId} from "react"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"
import {
    EPaperGridDensity,
    getPaperGridRows,
    getResponsivePaperGridColumns,
} from "../../ballot/paperGrid"

export const PAPER_INK = "#151515"
export const PAPER_RULE = "#333333"
export const PAPER_HEADING_BORDER = "#222222"
export const PAPER_HEADING_BACKGROUND = "#d5e5cb"
export const PAPER_FONT_FAMILY = "Arial, Helvetica, sans-serif"

interface IDensityMetrics {
    fontSize: string
    minHeight: string
    markSize: string
    paddingBlock: string
}

const DENSITY_METRICS: Record<EPaperGridDensity, IDensityMetrics> = {
    [EPaperGridDensity.REGULAR]: {
        fontSize: "14px",
        minHeight: "44px",
        markSize: "14px",
        paddingBlock: "6px",
    },
    [EPaperGridDensity.COMPACT]: {
        fontSize: "12px",
        minHeight: "36px",
        markSize: "12px",
        paddingBlock: "4px",
    },
    [EPaperGridDensity.DENSE]: {
        fontSize: "11px",
        minHeight: "24px",
        markSize: "11px",
        paddingBlock: "2px",
    },
}

export const PaperBallotHeading = styled("div")`
    margin-top: 25.5px;
    padding: 8px;
    border: 0.75pt solid ${PAPER_HEADING_BORDER};
    background-color: ${PAPER_HEADING_BACKGROUND};
    color: ${PAPER_INK};
    font-family: ${PAPER_FONT_FAMILY};
    text-align: center;

    .contest-title {
        margin: 0;
        justify-content: center;
        font-family: ${PAPER_FONT_FAMILY};
        font-size: 1.25rem;
        font-weight: bold;
        text-transform: uppercase;
        color: ${PAPER_INK};
    }

    .contest-title-text {
        flex-grow: 0;
    }

    .contest-paper-subtitle {
        display: block;
        margin-top: 2px;
        font-size: 0.8125rem;
        font-weight: bold;
    }
`

const columnsRule = (columns: number, candidateCount: number): string => `
    grid-template-columns: repeat(${columns}, minmax(0, 1fr));
    grid-template-rows: repeat(${getPaperGridRows(candidateCount, columns)}, auto);
`

export const PaperBallotGrid = styled("ul", {
    shouldForwardProp: (prop) => prop !== "columns" && prop !== "candidateCount",
})<{columns: number; candidateCount: number}>`
    display: grid;
    grid-auto-flow: column;
    list-style: none;
    margin: 0;
    padding: 0;
    border-top: 0.6pt solid ${PAPER_RULE};
    border-left: 0.6pt solid ${PAPER_RULE};
    background-color: #ffffff;
    color: ${PAPER_INK};
    font-family: ${PAPER_FONT_FAMILY};
    ${({columns, candidateCount}) =>
        columnsRule(getResponsivePaperGridColumns(columns).small, candidateCount)}

    @media (min-width: ${({theme}) => theme.breakpoints.values.md}px) {
        ${({columns, candidateCount}) =>
            columnsRule(getResponsivePaperGridColumns(columns).medium, candidateCount)}
    }

    @media (min-width: ${({theme}) => theme.breakpoints.values.lg}px) {
        ${({columns, candidateCount}) =>
            columnsRule(getResponsivePaperGridColumns(columns).large, candidateCount)}
    }
`

const PaperCell = styled("li", {
    shouldForwardProp: (prop) => prop !== "density" && prop !== "isDisabled",
})<{density: EPaperGridDensity; isDisabled: boolean}>`
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    min-height: ${({density}) => DENSITY_METRICS[density].minHeight};
    padding: ${({density}) => DENSITY_METRICS[density].paddingBlock} 4px
        ${({density}) => DENSITY_METRICS[density].paddingBlock}
        calc(${({density}) => DENSITY_METRICS[density].markSize} + 10px);
    border-right: 0.6pt solid ${PAPER_RULE};
    border-bottom: 0.6pt solid ${PAPER_RULE};
    font-size: ${({density}) => DENSITY_METRICS[density].fontSize};
    font-weight: bold;
    line-height: 1.15;
    text-transform: uppercase;
    overflow-wrap: anywhere;
    ${({isDisabled}) => (isDisabled ? "opacity: 0.5;" : "")}

    &:focus-within {
        outline: 2px solid ${PAPER_INK};
        outline-offset: -3px;
    }

    .paper-candidate-mark {
        position: absolute;
        left: 4px;
        top: 50%;
        width: ${({density}) => DENSITY_METRICS[density].markSize};
        height: ${({density}) => DENSITY_METRICS[density].markSize};
        transform: translateY(-50%);
        border: 1pt solid ${PAPER_HEADING_BORDER};
        border-radius: 50%;
        background-color: #ffffff;
        pointer-events: none;
    }

    &.paper-candidate-checked .paper-candidate-mark {
        background-color: ${PAPER_INK};
    }

    .paper-candidate-input {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        margin: 0;
        opacity: 0;
        cursor: pointer;
    }

    .paper-candidate-input:disabled {
        cursor: not-allowed;
    }

    .paper-candidate-subtitle {
        font-weight: normal;
    }

    .paper-candidate-details {
        position: relative;
        z-index: 1;
        margin-top: 4px;
        font-weight: normal;
        text-transform: none;
        line-height: 1.4;

        summary {
            cursor: pointer;
            text-decoration: underline;
        }

        .candidate-image {
            display: block;
            width: 64px;
            max-width: 100%;
            height: auto;
            margin-top: 4px;
        }

        a {
            color: inherit;
        }
    }

    .paper-candidate-write-in {
        position: relative;
        z-index: 1;
        width: 100%;
        margin-top: 2px;
        padding: 1px 2px;
        border: 0;
        border-bottom: 1px solid ${PAPER_INK};
        background: transparent;
        font: inherit;
        font-weight: normal;
        text-transform: none;
        color: ${PAPER_INK};
    }

    .paper-candidate-write-in[aria-invalid="true"] {
        border-bottom-color: ${({theme}) => theme.palette.errorColor};
    }
`

export interface PaperBallotCandidateProps {
    ballotNumber: number
    title: ReactNode
    subtitle?: string
    description?: ReactNode
    imageUrl?: string
    url?: string
    density: EPaperGridDensity
    checked: boolean
    isSelectable: boolean
    shouldDisable?: boolean
    setChecked?: (value: boolean) => void
    isWriteIn?: boolean
    writeInValue?: string
    setWriteInText?: (value: string) => void
    isInvalidWriteIn?: boolean
    writeInErrorId?: string
}

/**
 * One cell of a paper-style ballot: a fill-in circle and a numbered name.
 *
 * The checkbox is real and covers the whole cell, so a click anywhere on the
 * cell, a keyboard and assistive technology all reach the same control; only its
 * drawing is replaced by the circle.
 */
export const PaperBallotCandidate: React.FC<PaperBallotCandidateProps> = ({
    ballotNumber,
    title,
    subtitle,
    description,
    imageUrl,
    url,
    density,
    checked,
    isSelectable,
    shouldDisable,
    setChecked,
    isWriteIn,
    writeInValue,
    setWriteInText,
    isInvalidWriteIn,
    writeInErrorId,
}) => {
    const {t} = useTranslation()
    const generatedId = useId()
    const nameId = `${generatedId}-name`
    const writeInLabelId = `${generatedId}-writein-label`
    const detailsLabelId = `${generatedId}-details-label`
    const linkLabelId = `${generatedId}-link-label`
    const labelIds = isWriteIn ? `${writeInLabelId} ${nameId}` : nameId

    return (
        <PaperCell
            className={`candidate-item paper-candidate${checked ? " paper-candidate-checked" : ""}`}
            density={density}
            isDisabled={!!shouldDisable}
        >
            {isSelectable ? (
                <input
                    type="checkbox"
                    className="candidate-input paper-candidate-input"
                    aria-labelledby={labelIds}
                    checked={checked}
                    disabled={shouldDisable}
                    onChange={(event) => setChecked?.(event.target.checked)}
                />
            ) : null}
            <span className="paper-candidate-mark" aria-hidden="true" />
            <span className="candidate-title paper-candidate-name" id={nameId}>
                {`${ballotNumber}. `}
                {title}
                {subtitle ? (
                    <span className="candidate-subtitle paper-candidate-subtitle">
                        {` (${subtitle})`}
                    </span>
                ) : null}
            </span>
            {description || imageUrl || url ? (
                <details className="paper-candidate-details">
                    <summary aria-labelledby={`${detailsLabelId} ${nameId}`}>
                        <span id={detailsLabelId}>{t("candidate.moreInformationLink")}</span>
                    </summary>
                    {imageUrl ? <img className="candidate-image" src={imageUrl} alt="" /> : null}
                    {description ? (
                        <div className="candidate-description">{description}</div>
                    ) : null}
                    {url ? (
                        <a
                            className="candidate-link"
                            href={url}
                            target="_blank"
                            rel="noopener noreferrer"
                            aria-labelledby={`${linkLabelId} ${nameId}`}
                        >
                            <span id={linkLabelId}>{t("candidate.moreInformationLink")}</span>
                        </a>
                    ) : null}
                </details>
            ) : null}
            {isWriteIn ? (
                <>
                    <VisuallyHidden className="candidate-write-in-label" id={writeInLabelId}>
                        {t("a11y.writeInFor")}
                    </VisuallyHidden>
                    {isSelectable ? (
                        <input
                            type="text"
                            className="candidate-writein-textfield paper-candidate-write-in"
                            placeholder={t("candidate.writeInsPlaceholder")}
                            value={writeInValue ?? ""}
                            aria-labelledby={`${writeInLabelId} ${nameId}`}
                            aria-invalid={!!isInvalidWriteIn}
                            aria-describedby={
                                isInvalidWriteIn && writeInErrorId ? writeInErrorId : undefined
                            }
                            onChange={(event) => setWriteInText?.(event.target.value)}
                        />
                    ) : writeInValue ? (
                        <span className="paper-candidate-write-in-value">{writeInValue}</span>
                    ) : null}
                </>
            ) : null}
        </PaperCell>
    )
}
