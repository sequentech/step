// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETallyType} from "@/types/ceremonies"

export interface BallotImagesReportContext {
    miruExportEnabled: boolean
    tallyType?: string | null
    canGenerateReports: boolean
}

/**
 * The ballot-images report is one of the Miru exports of a results tally, and
 * generating it needs report-generate.
 */
export const offersBallotImagesReport = ({
    miruExportEnabled,
    tallyType,
    canGenerateReports,
}: BallotImagesReportContext): boolean =>
    miruExportEnabled && tallyType !== ETallyType.INITIALIZATION_REPORT && canGenerateReports
