// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The portal runs offline (the airgap release) as well as hosted, and sends
// nothing to third parties either way: react-admin's <Admin> reports usage to
// react-admin-telemetry.marmelab.com unless told not to. The airgap rehearsal
// saw the browser try it.
import {readFileSync} from "fs"
import {join} from "path"

it("the portal's react-admin root sends no telemetry", () => {
    const app = readFileSync(join(__dirname, "App.tsx"), "utf8")
    const admin = app.match(/<Admin\b[^>]*>/)
    expect(admin).not.toBeNull()
    expect(admin![0]).toMatch(/\bdisableTelemetry\b/)
})
