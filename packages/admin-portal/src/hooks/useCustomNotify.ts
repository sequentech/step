// SPDX-FileCopyrightText: 2025 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"

/**
 * A custom hook that provides a notification function with translation support.
 *
 * This hook wraps the `useNotify` function and integrates it with the `useTranslation`
 * hook to allow for localized notification messages. It ensures that the message
 * is not translated multiple times by providing a fallback message argument.
 *
 * @returns A function to trigger notifications with a translated message.
 *
 * The returned function accepts the following parameters:
 *
 * @param key - The translation key for the notification message.
 * @param options - An object containing notification options.
 * @param options.type - The type of notification, either "success" or "error".
 */
export const useCustomNotify = () => {
    const notify = useNotify()
    const {t} = useTranslation()

    return (key: string, options: {type: "success" | "error"}) => {
        notify(t(key), {
            ...options,
            messageArgs: {_: t(key)}, // Prevent double translation
        })
    }
}
