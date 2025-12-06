// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useFormState} from "react-hook-form"

export const DebugErrors: React.FC = () => {
    const {errors} = useFormState()
    console.log("Form Errors →", errors)
    return null
}
