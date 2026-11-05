// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {generateRowClickHandler, stringifyFields} from "./RowClickService"

jest.mock("@sequentech/ui-core", () => jest.requireActual("../../../ui-core/src/utils/typechecks"))

const AREA = "40000000-0000-4000-8000-000000000001"
const EVENT = "20000000-0000-4000-8000-000000000001"
const record = {id: AREA, election_event_id: EVENT, name: "North", parent_id: null}

describe("row click links", () => {
    it("opens the record's edit route filtered by the chosen fields", () => {
        const onClick = generateRowClickHandler(["election_event_id"])
        expect(onClick(AREA, "sequent_backend_area", record)).toBe(
            `/sequent_backend_area/${AREA}?filter={"election_event_id":"${EVENT}"}`
        )
    })

    it("opens the show route when asked", () => {
        const onClick = generateRowClickHandler(["election_event_id", "name"], true)
        expect(onClick(AREA, "sequent_backend_area", record)).toBe(
            `/sequent_backend_area/${AREA}/show?filter={"election_event_id":"${EVENT}","name":"North"}`
        )
    })

    it("keeps null fields and leaves out fields the record does not have", () => {
        expect(stringifyFields(record, ["parent_id", "tenant_id"])).toBe('{"parent_id":null}')
        expect(stringifyFields(record, [])).toBe("{}")
    })
})
