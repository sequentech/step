// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {createElectionEvent} from "./election-event/create"
import {deleteElectionEvent} from "./election-event/delete"

exports.command = function () {
    this.elements(
        "css selector",
        `a[title = '${createElectionEvent.config.electionEvent.name}']`,
        function (testElections) {
            console.log({testElections})
            testElections.value.forEach((i) => deleteElectionEvent.deleteElectionEvent(browser))
        }
    )

    return this
}
