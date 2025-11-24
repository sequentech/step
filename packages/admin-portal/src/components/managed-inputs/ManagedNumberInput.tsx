// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {NumberInput} from "react-admin"
import {useWatch} from "react-hook-form"

interface ManagedNumberInputProps {
    source: string
    label: string
    defaultValue: number
    sourceToWatch: string
    isDisabled: (selectedPolicy: any) => boolean
    min?: number
}

export const ManagedNumberInput = ({
    source,
    label,
    defaultValue,
    sourceToWatch,
    isDisabled,
    min,
}: ManagedNumberInputProps) => {
    const selectedPolicy = useWatch({name: sourceToWatch})

    return (
        <NumberInput
            source={source}
            disabled={isDisabled(selectedPolicy)}
            label={label}
            min={min}
            defaultValue={defaultValue}
            style={{flex: 1}}
        />
    )
}
