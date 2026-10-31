// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Data a screen story loads. Screen story files export one story for each
 * state the screen distinguishes, so that every state has a stable story ID.
 */
export enum EStoryDataState {
    LOADING = "loading",
    EMPTY = "empty",
    POPULATED = "populated",
    ERROR = "error",
}

/** A response that never arrives, which keeps a screen in its loading state. */
export const pending = <T>(): Promise<T> => new Promise<T>(() => undefined)
