// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
export const IVR_ENTITY_I18N_ANNOTATION = "ivr:i18n" as const
export const IVR_CONFIG_ANNOTATION = "ivr:config" as const
export const IVR_PROMPTS_ANNOTATION = "ivr:prompts" as const

type EntityAnnotations = Record<string, unknown> | null | undefined
type IvrEntityI18nAnnotation = Record<string, IvrEntityI18nAnnotationContent>
interface IvrEntityI18nAnnotationContent {
    prompt?: string
}

export const parseIvrEntityAnnotations = (
    annotations: EntityAnnotations
): Record<string, unknown> => {
    const obj: Record<string, unknown> = {...(annotations ?? {})}
    const ivrRaw = obj[IVR_ENTITY_I18N_ANNOTATION]
    let parsed: IvrEntityI18nAnnotation = {}
    if (typeof ivrRaw === "string") {
        try {
            parsed = JSON.parse(ivrRaw) as IvrEntityI18nAnnotation
        } catch (e) {
            console.error("Failed to parse ivr entity annotations", e)
        }
    } else if (ivrRaw) {
        console.error("Unexpected type of ivr entity annotation", typeof ivrRaw, ivrRaw)
    }
    obj[IVR_ENTITY_I18N_ANNOTATION] = parsed

    return obj
}

export const serializeIvrEntityAnnotations = (
    annotations: EntityAnnotations
): Record<string, string> => {
    const result: Record<string, unknown> = {...(annotations ?? {})}
    const ivr = result[IVR_ENTITY_I18N_ANNOTATION]
    if (ivr && typeof ivr === "object") {
        result[IVR_ENTITY_I18N_ANNOTATION] = JSON.stringify(ivr)
    }

    // TODO: Api contract requires annotations to be string:string,
    //  but we only update the ivr object here to avoid affecting elsewhere.
    return result as Record<string, string>
}
