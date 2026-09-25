// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Requests that no mock expected. A test fixture asserts that the log is empty
 * once the test body has finished, so an unexpected request fails the test even
 * when the page recovered from it.
 */
export class ViolationLog {
    private readonly messages: string[] = []

    add(message: string): void {
        this.messages.push(message)
    }

    list(): string[] {
        return [...this.messages]
    }

    clear(): void {
        this.messages.length = 0
    }
}
