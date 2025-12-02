// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useLocation} from "react-router-dom"

export function useUrlParams() {
    const [, type, id] = useLocation().pathname.split("/")

    const ids = {
        election_event_id: type === "sequent_backend_election_event" ? id : undefined,
        election_id: type === "sequent_backend_election" ? id : undefined,
        contest_id: type === "sequent_backend_contest" ? id : undefined,
        candidate_id: type === "sequent_backend_candidate" ? id : undefined,
    }

    return ids
}
