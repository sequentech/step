// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useRef} from "react"
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

const noop = () => undefined

const PhoneInput = ({
    handlePhoneNumberChange,
    label,
    fullWidth,
    initialValue,
    disabled,
}: PhoneInputProps) => {
    const handlePhoneNumberChangeRef = useRef(handlePhoneNumberChange)
    handlePhoneNumberChangeRef.current = handlePhoneNumberChange

    // intl-tel-input re-subscribes and invokes its update callback whenever
    // any callback prop changes, so keep every callback identity stable.
    const onChangeNumber = useCallback((number: string) => {
        handlePhoneNumberChangeRef.current(number)
    }, [])

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
                onChangeCountry={noop}
                onChangeValidity={noop}
                onChangeErrorCode={noop}
                initialValue={initialValue}
                disabled={disabled}
            />
        </Box>
    )
}

export default PhoneInput
