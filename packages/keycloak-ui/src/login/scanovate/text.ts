// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {dynamicMessageLanguage, messageLanguage, type I18n, type MessageKey} from "../i18n"
import type {KcContext} from "../KcContext"
import type {TemplateExtras, TemplateLabel} from "../Template"

export const ENROLLMENT_STEPS = 4

export enum EnrollmentStep {
    Details = 1,
    VerifyIdentity = 3,
    Confirm = 4,
}

const STEP_NAMES: Record<EnrollmentStep, MessageKey> = {
    [EnrollmentStep.Details]: "enrollmentStepDetails",
    [EnrollmentStep.VerifyIdentity]: "scanovateStepVerifyIdentity",
    [EnrollmentStep.Confirm]: "scanovateStepConfirm",
}

export type Text = (key: MessageKey, ...args: string[]) => TemplateLabel

export function textFor(kcContext: KcContext, i18n: I18n): Text {
    return (key, ...args) => ({
        text: i18n.msgStr(key, ...args),
        lang: messageLanguage(kcContext, i18n, key),
    })
}

// Message keys chosen at run time (document types, stored attributes, server errors).
export function dynamicText(kcContext: KcContext, i18n: I18n, key: string): TemplateLabel {
    return {
        text: i18n.advancedMsgStr(key),
        lang: dynamicMessageLanguage(kcContext, i18n, key),
    }
}

export function hasMessage(i18n: I18n, key: string): boolean {
    return i18n.advancedMsgStr(key) !== key
}

// The authenticator sends this when the voter's document type is unknown.
export const DEFAULT_DOCUMENT_TYPE = "default"

export function documentName(
    kcContext: KcContext,
    i18n: I18n,
    documentType: string | undefined
): TemplateLabel {
    const key = `scanovateDocumentType.${documentType}`
    if (
        documentType !== undefined &&
        documentType !== "" &&
        documentType !== DEFAULT_DOCUMENT_TYPE &&
        hasMessage(i18n, key)
    ) {
        return dynamicText(kcContext, i18n, key)
    }
    return textFor(kcContext, i18n)("scanovateDocumentGeneric")
}

export function capitalized(label: TemplateLabel): TemplateLabel {
    return {
        ...label,
        text: label.text.charAt(0).toLocaleUpperCase(label.lang) + label.text.slice(1),
    }
}

export function enrollmentFrame(
    kcContext: KcContext,
    i18n: I18n,
    step: EnrollmentStep
): Pick<TemplateExtras, "eyebrow" | "progress"> {
    const text = textFor(kcContext, i18n)
    const eyebrow = text(
        "scanovateStepEyebrow",
        String(step),
        String(ENROLLMENT_STEPS),
        i18n.msgStr(STEP_NAMES[step])
    )
    return {
        eyebrow,
        progress: {step, total: ENROLLMENT_STEPS, label: text("scanovateProgressLabel")},
    }
}

// Every step is done: the pages that end the enrollment.
export function finishedFrame(
    kcContext: KcContext,
    i18n: I18n,
    outcome: MessageKey
): Pick<TemplateExtras, "eyebrow" | "progress"> {
    const text = textFor(kcContext, i18n)
    return {
        eyebrow: text(outcome),
        progress: {
            step: ENROLLMENT_STEPS + 1,
            total: ENROLLMENT_STEPS,
            label: text("scanovateProgressLabel"),
        },
    }
}
