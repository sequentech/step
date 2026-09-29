// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as SequentCore from "sequent-core"
import {createLocalValidator, type ILocalValidatorOptions} from "./localValidator"
import type {EMonitoringConfigKind} from "./types"

/** The local validator backed by the portal's sequent-core WebAssembly module. */
export const sequentCoreValidator = (
    kind: EMonitoringConfigKind,
    options?: ILocalValidatorOptions
) => createLocalValidator(SequentCore, kind, options)
