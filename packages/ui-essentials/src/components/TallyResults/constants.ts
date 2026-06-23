// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const MAX_CANDIDATES_REPRESENTED = 5

export const DATA_GRID_INITIAL_STATE = {
    pagination: {
        paginationModel: {
            pageSize: 10,
        },
    },
}

export const DATA_GRID_PAGE_SIZE_OPTIONS = [10, 20, 50, 100]

export const RESPONSIVE_PIE_OPTIONS = [
    {
        breakpoint: 480,
        options: {
            chart: {
                width: 200,
            },
            legend: {
                position: "bottom",
            },
        },
    },
]

export const CANDIDATE_CHART_COLORS = [
    "#008FFBFF",
    "#FF0000",
    "#dfdf01ff",
    "#079107ff",
    "#FF8000",
    "#706565ff",
]

export const PREFERENTIAL_ROUND_COLUMN_WIDTH = 320
