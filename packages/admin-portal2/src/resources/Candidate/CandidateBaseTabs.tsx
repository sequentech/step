// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ShowBase} from "react-admin"
import {CandidateTabs} from "./CandidateTabs"

export const CandidateBaseTabs: React.FC = () => {
    return (
        <ShowBase>
            <CandidateTabs />
        </ShowBase>
    )
}
