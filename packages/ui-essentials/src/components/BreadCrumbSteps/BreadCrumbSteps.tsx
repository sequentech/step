// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React from "react"
import {styled} from "@mui/material/styles"
import styledEmotion from "@emotion/styled"
import {useTranslation} from "react-i18next"

const StepsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 10px;
    width: 100%;
    align-items: center;
`

interface StepNumberProps {
    isselected: string
    warning: string
}

const StepNumber = styled(Box)<StepNumberProps>`
    display: flex;
    width: 22px;
    height: 22px;
    font-size: 15px;
    font-style: normal;
    font-weight: ${({isselected}) => ("true" === isselected ? "600" : "400")};
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 8px;
    border-radius: 4px;
    ${({isselected}) => ("true" === isselected ? "" : "opacity: 0.5;")}
    border: ${({isselected, theme}) =>
        "true" === isselected ? "inherit" : `1px solid ${theme.palette.brandColor}`};
    color: ${({isselected, theme}) =>
        "true" === isselected ? theme.palette.white : theme.palette.brandColor};
    background: ${({isselected, warning, theme}) =>
        "true" === isselected
            ? "true" === warning
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
}

const StepLabel = styledEmotion(Box)<StepLabelProps>`
    color: ${({isselected, theme}) =>
        "true" === isselected
            ? theme.palette.customGrey.contrastText
            : theme.palette.customGrey.main};
`

interface StepProps {
    label: string
    isSelected: boolean
    isLast: boolean
    index: number
    warning?: boolean
}

const Step: React.FC<StepProps> = ({label, isSelected, isLast, index, warning}) => {
    const {t} = useTranslation()

    return (
        <>
            <StepNumber isselected={String(isSelected)} warning={String(!!warning)}>
                {index + 1}
            </StepNumber>
            <StepLabel
                isselected={String(isSelected)}
                className={isSelected ? "selected" : "not-selected"}
            >
                {t(label)}
            </StepLabel>
            {isLast ? null : <StepSeparator />}
        </>
    )
}

interface BreadCrumbStepsProps {
    labels: Array<string>
    selected: number
    warning?: boolean
}

const BreadCrumbSteps: React.FC<BreadCrumbStepsProps> = ({labels, selected, warning}) => (
    <StepsContainer>
        {labels.map((label, index) => (
            <Step
                key={index}
                label={label}
                index={index}
                isSelected={index === selected}
                isLast={index + 1 === labels.length}
                warning={warning}
            />
        ))}
    </StepsContainer>
)

export default BreadCrumbSteps
