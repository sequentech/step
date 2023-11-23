// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import styledEmotion from "@emotion/styled"
import {useTranslation} from "react-i18next"

export enum BreadCrumbStepsVariant {
    Default = "default",
    Circle = "circle",
}

const StepsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 10px;
    width: 100%;
    align-items: center;
`

interface StepNumberProps {
    variant: BreadCrumbStepsVariant
    isSelected: boolean
    warning: boolean
}

const StepNumber = styled(Box)<StepNumberProps>`
    display: flex;
    width: 22px;
    height: 22px;
    font-size: 15px;
    font-style: normal;
    font-weight: ${({isSelected}) => (isSelected ? "600" : "400")};
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 8px;
    ${({variant}) =>
        variant === BreadCrumbStepsVariant.Default ? "border-radius: 4px;" : "border-radius: 100%;"}
    ${({isSelected}) => !isSelected && "opacity: 0.5;"}
    border: ${({isSelected, theme}) =>
        isSelected ? "inherit" : `1px solid ${theme.palette.brandColor}`};
    color: ${({isSelected, theme}) =>
        isSelected ? theme.palette.white : theme.palette.brandColor};
    background: ${({isSelected, warning, theme}) =>
        isSelected ? (warning ? theme.palette.errorColor : theme.palette.brandColor) : "inherit"};
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
    isSelected: boolean
}

const StepLabel = styledEmotion(Box)<StepLabelProps>`
    color: ${({isSelected, theme}) =>
        isSelected ? theme.palette.customGrey.contrastText : theme.palette.customGrey.main};
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
        <>
            <StepNumber variant={variant} isSelected={isSelected || colorStep} warning={!!warning}>
                {index + 1}
            </StepNumber>
            <StepLabel
                isSelected={isSelected || colorStep}
                className={isSelected ? "selected" : "not-selected"}
            >
                {t(label)}
            </StepLabel>
            {isLast ? null : <StepSeparator />}
        </>
    )
}

interface BreadCrumbStepsProps {
    variant?: BreadCrumbStepsVariant
    labels: Array<string>
    selected: number
    warning?: boolean
    colorPreviousSteps?: boolean
}

export default function BreadCrumbSteps({
    variant = BreadCrumbStepsVariant.Default,
    labels,
    selected,
    warning,
    colorPreviousSteps = false,
}: BreadCrumbStepsProps) {
    return (
        <StepsContainer>
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
