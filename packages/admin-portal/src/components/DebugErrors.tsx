// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useFormState} from "react-hook-form"

export const DebugErrors: React.FC = () => {
    const {errors} = useFormState({
        // Only subscribe to errors to optimize re-renders
        control: undefined,
    })

    React.useEffect(() => {
        if (errors && Object.keys(errors).length > 0) {
            console.group("Form Validation Errors")
            console.log("Current errors:", errors)
            console.groupEnd()
        }
    }, [errors])

    return null
}
