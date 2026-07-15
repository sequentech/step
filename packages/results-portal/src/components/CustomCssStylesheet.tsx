// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"

export const CustomCssStylesheet: React.FC<{css: string}> = ({css}) =>
    css ? (
        <style className="seq-results-portal-custom-css" data-seq-results-custom-css="active">
            {css}
        </style>
    ) : null
