// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IElectionStatus, EVotingStatus} from "@sequentech/ui-core"

export const getVotingStatus = (data?: IElectionStatus): EVotingStatus => {
    return data?.voting_status || EVotingStatus.NOT_STARTED
}
