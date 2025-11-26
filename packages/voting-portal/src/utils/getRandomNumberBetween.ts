// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getRandomNumberBetween = (n: number, m: number) => {
    return Math.floor(Math.random() * (m - n + 1)) + n
}
