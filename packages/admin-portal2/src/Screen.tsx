// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Dialog} from "@sequentech/ui-essentials"

let Screen: React.FC = () => {
    return <div>
        Hello FFélix
        <Dialog open={true} handleClose={() => undefined} title="my title" ok="Ok">
            My dialog
        </Dialog>
    </div>
}

export default Screen