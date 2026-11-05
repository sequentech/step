// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createStorybookTests} from "../test-support/storybook/vitest"

export default createStorybookTests(new URL(".storybook", import.meta.url))
