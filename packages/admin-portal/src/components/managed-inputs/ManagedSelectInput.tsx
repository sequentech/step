// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {required, SelectInput} from "react-admin"
import {useWatch} from "react-hook-form"

interface ManagedSelectInputProps {
    source: string
    label: string
    choices: any[]
    defaultValue: any
    sourceToWatch?: string
    isDisabled?: (selectedPolicy: any) => boolean
}

export const ManagedSelectInput = ({
    source,
    label,
    choices,
    defaultValue,
    sourceToWatch,
    isDisabled,
}: ManagedSelectInputProps) => {
    const sourceToWatchStatus = useWatch({name: sourceToWatch ?? ""})

    return (
        <SelectInput
            source={source}
            choices={choices}
            label={label}
            disabled={isDisabled?.(sourceToWatchStatus)}
            defaultValue={defaultValue}
            validate={required()}
            style={{flex: 1}}
        />
    )
}
