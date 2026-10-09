// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState} from "react"
import {loadScript, loadStylesheet} from "./assets"
import {timezoneCountry} from "./profile"

type PhoneWidget = {
    getNumber: () => string
    destroy: () => void
}

type CountryLookup = (success: (country: string) => void, failure: () => void) => void

declare global {
    interface Window {
        // intl-tel-input, from sequent-theme's resources.
        intlTelInput?: (input: HTMLInputElement, options: Record<string, unknown>) => PhoneWidget
        // The theme's time zone to country map, a JSON string.
        data?: string
    }
}

const WIDGET = "intl-tel-input-23.3.2"

// The pattern describes the stored number; with a separate dial code the input
// only holds the national part. The browser checks the whole number instead.
function patternError(pattern: string, number: string): string {
    const probe = document.createElement("input")
    probe.type = "text"
    probe.required = true
    probe.pattern = pattern
    probe.value = number
    return probe.validity.valid ? "" : probe.validationMessage
}

const lookupCountry: CountryLookup = (success, failure) => {
    const country = timezoneCountry(window.data, Intl.DateTimeFormat().resolvedOptions().timeZone)
    if (country === undefined) failure()
    else success(country)
}

export type PhoneFieldProps = {
    id: string
    // The theme's resources, which hold the widget.
    resources: string
    defaultValue: string
    pattern?: string
    placeholder?: string
    required: boolean
    readOnly: boolean
    disabled: boolean
    invalid: boolean
    describedBy?: string
    inputRef: (input: HTMLInputElement | null) => void
}

// A phone number with its country prefix. The number is posted under the
// attribute's name, the country under country_code. Without the widget the
// plain input posts what the voter types.
export default function PhoneField(props: PhoneFieldProps) {
    const {id, resources, defaultValue, pattern, placeholder, required, readOnly, disabled} = props
    const {invalid, describedBy, inputRef} = props
    const input = useRef<HTMLInputElement | null>(null)
    // With the widget, the number is posted by its hidden input. React writes
    // the input's name on every update, so it has to know which one it is.
    const [enhanced, setEnhanced] = useState(false)

    useEffect(() => {
        const element = input.current
        if (element === null) return
        let widget: PhoneWidget | undefined
        let cancelled = false
        const validate = () => {
            if (widget === undefined || pattern === undefined) return
            element.setCustomValidity(
                element.value === "" ? "" : patternError(pattern, widget.getNumber())
            )
        }
        const start = () => {
            if (cancelled || window.intlTelInput === undefined) return
            setEnhanced(true)
            widget = window.intlTelInput(element, {
                initialCountry: "auto",
                separateDialCode: true,
                customPlaceholder: (example: string) => example.replace(/\d/g, "0"),
                hiddenInput: () => ({phone: id, country: "country_code"}),
                geoIpLookup: lookupCountry,
            })
            element.addEventListener("input", validate)
            element.addEventListener("countrychange", validate)
            validate()
        }
        if (window.intlTelInput !== undefined) {
            start()
        } else {
            loadStylesheet(`${resources}/${WIDGET}/css/intlTelInput.css`)
            void Promise.all([
                loadScript(`${resources}/${WIDGET}/js/intlTelInputWithUtils.min.js`),
                // Without the map the widget starts with no country.
                loadScript(`${resources}/js/timezone-countrycode-data.js`).catch(() => undefined),
            ]).then(start, () => undefined)
        }
        return () => {
            cancelled = true
            element.removeEventListener("input", validate)
            element.removeEventListener("countrychange", validate)
            element.setCustomValidity("")
            widget?.destroy()
            setEnhanced(false)
        }
    }, [id, resources, pattern])

    // A locked phone is sent empty.
    useEffect(() => {
        if (readOnly && input.current !== null) input.current.value = ""
    }, [readOnly])

    return (
        <div className="auth-phone">
            <input
                ref={(element) => {
                    input.current = element
                    inputRef(element)
                }}
                type="tel"
                id={id}
                name={enhanced ? `${id}-input` : id}
                className="auth-input"
                defaultValue={defaultValue}
                pattern={enhanced ? undefined : pattern}
                placeholder={placeholder}
                required={required}
                readOnly={readOnly}
                disabled={disabled}
                aria-invalid={invalid || undefined}
                aria-describedby={describedBy}
            />
        </div>
    )
}
