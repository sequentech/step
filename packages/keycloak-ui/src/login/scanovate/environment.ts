// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {createContext, useContext} from "react"
import {browserServices} from "./media"
import type {CaptureServices, CaptureStep} from "./types"

export type CaptureEnvironment = {
    services: CaptureServices
    // Previews open the camera directly at this step.
    startAt?: CaptureStep
}

export const CaptureEnvironmentContext = createContext<CaptureEnvironment>({
    services: browserServices,
})

export function useCaptureEnvironment(): CaptureEnvironment {
    return useContext(CaptureEnvironmentContext)
}
