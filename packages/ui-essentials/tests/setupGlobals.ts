// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {TextDecoder, TextEncoder} from "node:util"

// React Router uses these browser APIs during module initialization. jsdom
// does not supply them, so install Node's implementations before imports run.
Object.assign(globalThis, {TextDecoder, TextEncoder})
