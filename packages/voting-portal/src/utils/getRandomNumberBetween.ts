// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getRandomNumberBetween = (n: number, m: number) => {
    return Math.floor(Math.random() * (m - n + 1)) + n
}
