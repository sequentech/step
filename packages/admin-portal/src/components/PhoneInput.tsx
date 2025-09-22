// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import IntlTelInput from "intl-tel-input/react"
import {Box, InputLabel} from "@mui/material"
import {data} from "../lib/timezone-countrycode-data"

interface PhoneInputProps {
    handlePhoneNumberChange: (number: string) => void
    label?: string
    fullWidth?: boolean
    initialValue?: string
    disabled?: boolean
}
const PhoneInput = ({
    handlePhoneNumberChange,
    label,
    fullWidth,
    initialValue,
    disabled,
}: PhoneInputProps) => {
    const [isValid, setIsValid] = useState<boolean | null>(null)

    const onChangeNumber = (number: string) => {
        handlePhoneNumberChange(number)
    }
    return (
        <Box sx={{margin: "16px 0", ...(fullWidth && {width: "100%"})}}>
            <InputLabel>{label}</InputLabel>
            <IntlTelInput
                initOptions={{
                    utilsScript: process.env.PUBLIC_URL + "/intl-tel-input/phoneInput.js",
                    initialCountry: "auto",
                    separateDialCode: true,
                    geoIpLookup: (success, failure) => {
                        const userTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
                        let countryCode = data[userTimeZone].toString()
                        if (countryCode) {
                            return success(countryCode)
                        }
                        return failure()
                    },
                }}
                onChangeNumber={onChangeNumber}
                onChangeValidity={setIsValid}
                initialValue={initialValue}
                disabled={disabled}
            />
        </Box>
    )
}

export default PhoneInput
