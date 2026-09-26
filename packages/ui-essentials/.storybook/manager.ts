// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {addons} from "storybook/manager-api"

// A portal Storybook that was not running when this one started fails the
// server's check, so the browser requests its index with credentials, which the
// portal's wildcard CORS response rejects. Checking it again as a server-checked
// reference omits the credentials; this happens once the first check settles and
// whenever the window regains focus, so portals started later appear on return.
addons.register("sequent/portal-refs", (api) => {
    const reload = async () => {
        for (const ref of Object.values(api.getRefs())) {
            if (ref.index) continue
            await api.checkRef({...ref, type: "server-checked"})
            // A successful check keeps the earlier failure unless it is cleared.
            if (api.getRefs()[ref.id]?.index) await api.updateRef(ref.id, {indexError: undefined})
        }
    }
    const firstCheck = setInterval(() => {
        if (Object.values(api.getRefs()).every((ref) => ref.index || ref.indexError)) {
            clearInterval(firstCheck)
            void reload()
        }
    }, 250)
    window.addEventListener("focus", () => void reload())
})
