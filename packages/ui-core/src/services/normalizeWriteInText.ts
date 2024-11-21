// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const normalizeWriteInText = (input: string): string => {
    // normalize the string to form NFC and replace diacritics with their base characters
    let cleaned = input.normalize("NFD").replace(/[\u0300-\u036f]/g, "")

    // remove all tildes
    cleaned = cleaned.replace(/~/g, "")

    // upper case everything
    cleaned = cleaned.toUpperCase()

    // remove invalid characters
    cleaned = cleaned.replace(/[^A-Za-z\s,.()]/g, "")

    return cleaned
}
