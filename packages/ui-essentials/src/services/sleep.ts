// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const sleep = async (ms: number): Promise<undefined> => {
    return new Promise<undefined>((resolve) => setTimeout(() => resolve(undefined), ms))
}
