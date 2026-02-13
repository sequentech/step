// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Helper function to generate a random password
export const generateRandomPassword = (length = 12) => {
    const charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_."
    const charsetLength = charset.length

    // Array to hold random values
    const randomValues = new Uint8Array(length)

    // Fill array with secure random values
    crypto.getRandomValues(randomValues)

    let password = ""
    for (let i = 0; i < length; i++) {
        // Map random value to charset index
        password += charset[randomValues[i] % charsetLength]
    }

    return password
}
