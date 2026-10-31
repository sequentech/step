// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useRef, useState, type FormEvent} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import {ArrowIcon, IdCardIcon} from "../icons"
import type {ScanovateStoredAttribute} from "../KcContext"
import type {ScanovatePageProps} from "../scanovate/pageProps"
import {
    DEFAULT_DOCUMENT_TYPE,
    capitalized,
    EnrollmentStep,
    documentName,
    dynamicText,
    enrollmentFrame,
    textFor,
} from "../scanovate/text"

enum ConfirmationAction {
    Confirm = "confirm",
    Retry = "retry",
}

const DATE_TYPE = "date"
const ISO_DATE = /^(\d{4})-(\d{2})-(\d{2})$/

// Stored dates are yyyy-MM-dd; show them in the voter's language.
export function displayValue(attribute: ScanovateStoredAttribute, languageTag: string): string {
    const match = attribute.type === DATE_TYPE ? ISO_DATE.exec(attribute.value) : null
    if (match === null) return attribute.value
    const [, year, month, day] = match
    const date = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)))
    if (Number.isNaN(date.getTime())) return attribute.value
    try {
        return new Intl.DateTimeFormat(languageTag, {dateStyle: "long", timeZone: "UTC"}).format(
            date
        )
    } catch {
        return attribute.value
    }
}

export default function ScanovateConfirmation(
    props: ScanovatePageProps<"scanovate-confirmation.ftl">
) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {url, storedAttributes, documentType} = kcContext
    const [submitting, setSubmitting] = useState(false)
    const action = useRef<HTMLInputElement>(null)
    // Disabling the buttons can reach some browsers before they collect the form, which then
    // leaves out the clicked one: its action goes in a field of its own first.
    const onSubmit = (event: FormEvent<HTMLFormElement>) => {
        const submitter = (event.nativeEvent as SubmitEvent).submitter as HTMLButtonElement | null
        if (action.current !== null && submitter !== null) action.current.value = submitter.value
        setSubmitting(true)
    }
    const text = textFor(kcContext, i18n)
    const title = text("scanovateConfirmTitle")
    const document = documentName(kcContext, i18n, documentType)
    const lead = text("scanovateConfirmLead", document.text)
    const confirm = text("scanovateConfirmSubmit")
    const retry = text("scanovateConfirmRetry")
    const languageTag = i18n.currentLanguage.languageTag
    const rows = storedAttributes.map((attribute) => ({
        key: attribute.key,
        label: dynamicText(kcContext, i18n, attribute.key),
        value: displayValue(attribute, languageTag),
    }))
    if (documentType !== undefined && documentType !== DEFAULT_DOCUMENT_TYPE) {
        rows.push({
            key: "documentType",
            label: text("scanovateConfirmDocument"),
            value: capitalized(document).text,
        })
    }

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            {...enrollmentFrame(kcContext, i18n, EnrollmentStep.Confirm)}
            symbol={<IdCardIcon />}
            headerNode={title.text}
            titleLang={title.lang}
        >
            <p className="auth-lead" lang={lead.lang}>
                {lead.text}
            </p>
            <dl className="auth-summary">
                {rows.map((row) => (
                    <div key={row.key}>
                        <dt lang={row.label.lang}>{row.label.text}</dt>
                        <dd>{row.value}</dd>
                    </div>
                ))}
            </dl>
            <Box
                component="form"
                method="post"
                action={url.loginAction}
                className="auth-actions"
                onSubmit={onSubmit}
            >
                <input ref={action} type="hidden" name="action" />
                <Button
                    type="submit"
                    name="action"
                    value={ConfirmationAction.Confirm}
                    variant="contained"
                    fullWidth
                    className="auth-submit"
                    endIcon={<ArrowIcon />}
                    disabled={submitting}
                    lang={confirm.lang}
                >
                    {confirm.text}
                </Button>
                <Button
                    type="submit"
                    name="action"
                    value={ConfirmationAction.Retry}
                    variant="outlined"
                    fullWidth
                    disabled={submitting}
                    lang={retry.lang}
                >
                    {retry.text}
                </Button>
            </Box>
        </Template>
    )
}
