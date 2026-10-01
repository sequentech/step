// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// jsdom does not expose the WHATWG encoding classes react-router needs.
const {TextEncoder, TextDecoder} = require("util")

Object.assign(globalThis, {TextEncoder, TextDecoder})
