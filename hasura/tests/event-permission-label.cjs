// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The election event permission label, as every admin role's grants check it:
// an event without a label is open to every admin user of the tenant, a
// labelled one only to admin users whose permission labels hold it.
const eventLabel = {
    _or: [
        {permission_label: {_is_null: true}},
        {permission_label: {_in: "X-Hasura-Permission-Labels"}},
    ],
}

// The same check from a row that belongs to an event. Tenant-level rows have
// no event and stay open.
const viaEvent = {
    _or: [{election_event_id: {_is_null: true}}, {election_event: eventLabel}],
}

module.exports = {eventLabel, viaEvent}
