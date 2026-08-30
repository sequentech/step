// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"

export enum BreadCrumbStepsVariant {
    Default = "default",
    Circle = "circle",
}

const StepsContainer = styled("ol")`
    display: flex;
    flex-direction: row;
    gap: 10px;
    width: 100%;
    align-items: center;
    list-style: none;
    margin: 0;
    padding-inline-start: 0;
`

const StepItem = styled("li")<{islast: string}>`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 10px;
    ${({islast}) => (islast === "true" ? "" : "flex-grow: 2;")}
`

interface StepNumberProps {
    variant: BreadCrumbStepsVariant
    isselected: string
    warning: string
}

const StepNumber = styled(Box)<StepNumberProps>`
    display: flex;
    width: 22px;
    height: 22px;
    font-size: 15px;
    font-style: normal;
    font-weight: ${({isselected}) => (isselected === "true" ? "600" : "400")};
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 8px;
    ${({variant}) =>
        variant === BreadCrumbStepsVariant.Default ? "border-radius: 4px;" : "border-radius: 100%;"}
    ${({isselected}) => isselected !== "true" && "opacity: 0.8;"}
    border: ${({isselected, theme}) =>
        isselected === "true" ? "inherit" : `1px solid ${theme.palette.brandColor}`};
    color: ${({isselected, theme}) =>
        isselected === "true" ? theme.palette.white : theme.palette.brandColor};
    background: ${({isselected, warning, theme}) =>
        isselected === "true"
            ? warning === "true"
                ? theme.palette.errorColor
                : theme.palette.brandColor
            : "inherit"};
`

const StepSeparator = styled(Box)(
    ({theme}) => `
    border-top: dashed 1px #191D23;
    border-color: ${theme.palette.customGrey.contrastText};
    height: 1px;
    flex-grow: 2;
`
)

interface StepLabelProps {
    isselected: string
    iscurrent: string
}

const StepLabel = styled(Box)<StepLabelProps>`
    color: ${({isselected, theme}) =>
        isselected === "true"
            ? theme.palette.customGrey.contrastText
            : theme.palette.customGrey.main};

    /* On narrow screens only the current step's label is painted, but the other
       labels stay in the accessibility tree so the sequence is still readable.
       Uses the theme's own down("sm") query so the cut-over lands on the same
       pixel as the responsive display prop this replaced. */
    ${({theme}) => theme.breakpoints.down("sm")} {
        ${({iscurrent}) =>
            iscurrent === "true"
                ? ""
                : `
                    position: absolute;
                    width: 1px;
                    height: 1px;
                    padding: 0;
                    margin: -1px;
                    overflow: hidden;
                    clip: rect(0 0 0 0);
                    white-space: nowrap;
                    border: 0;
                `}
    }
`

interface StepProps {
    variant: BreadCrumbStepsVariant
    label: string
    isSelected: boolean
    isLast: boolean
    index: number
    warning?: boolean
    colorStep?: boolean
}

function Step({variant, label, isSelected, isLast, index, warning, colorStep = false}: StepProps) {
    const {t} = useTranslation()

    return (
        <StepItem islast={isLast.toString()} aria-current={isSelected ? "step" : undefined}>
            {/* Decorative: role="list" + <li> already gives AT the item's
                position (e.g. "1 of 4"), so reading this digit too doubled
                every step's announcement. */}
            <StepNumber
                className="step-number"
                variant={variant}
                isselected={(isSelected || colorStep).toString()}
                warning={(!!warning).toString()}
                aria-hidden="true"
            >
                {index + 1}
            </StepNumber>
            <StepLabel
                isselected={(isSelected || colorStep).toString()}
                iscurrent={isSelected.toString()}
                className={isSelected ? "selected" : "not-selected"}
            >
                {t(label)}
            </StepLabel>
            {isLast ? null : <StepSeparator className="step-separator" aria-hidden="true" />}
        </StepItem>
    )
}

interface BreadCrumbStepsProps {
    variant?: BreadCrumbStepsVariant
    labels: Array<string>
    selected: number
    warning?: boolean
    colorPreviousSteps?: boolean
    // Names the surrounding landmark, e.g. "Voting progress". Supplied by the
    // consuming portal because the same stepper describes different journeys.
    ariaLabel?: string
}

export default function BreadCrumbSteps({
    variant = BreadCrumbStepsVariant.Default,
    labels,
    selected,
    warning,
    colorPreviousSteps = false,
    ariaLabel,
}: BreadCrumbStepsProps) {
    // Named with aria-label rather than wrapped in a <nav>: the steps are labels,
    // not links, so a navigation landmark would advertise nothing navigable —
    // and consumers that pass no label would emit an unnamed landmark. The
    // explicit role="list" is needed because the <ol> is styled list-style: none,
    // which makes Safari and VoiceOver drop the list semantics.
    return (
        <StepsContainer className="step-container" role="list" aria-label={ariaLabel}>
            {labels.map((label, index) => (
                <Step
                    key={index}
                    variant={variant}
                    label={label}
                    index={index}
                    isSelected={index === selected}
                    colorStep={colorPreviousSteps ? index <= selected : false}
                    isLast={index + 1 === labels.length}
                    warning={warning}
                />
            ))}
        </StepsContainer>
    )
}
