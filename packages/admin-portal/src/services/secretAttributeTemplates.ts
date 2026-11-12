// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const escapeRegExp = (value: string): string => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")

/**
 * Virtual voter attributes the worker derives for the one-time PIN link. A
 * template using them gets the permission and audit of secret attributes.
 */
export const PIN_LINK_TEMPLATE_VARIABLES = ["pin_link_token", "pin_link_url"]

/**
 * Returns the secret attributes a communication actually uses, meaning those
 * referenced inside a Handlebars expression such as `{{user.reference}}` or
 * `{{lookup user.attributes "reference"}}`. A name that only appears in prose
 * is not declared, so it is neither decrypted nor sent to the worker.
 */
export const getReferencedSecretAttributeNames = (
    templateContents: string,
    secretAttributeNames: string[]
): string[] => {
    const expressions = templateContents.match(/{{[^}]*}}/g) ?? []
    return secretAttributeNames.filter((name) => {
        const reference = new RegExp(`(^|[^A-Za-z0-9_-])${escapeRegExp(name)}([^A-Za-z0-9_-]|$)`)
        return expressions.some((expression) => reference.test(expression))
    })
}
