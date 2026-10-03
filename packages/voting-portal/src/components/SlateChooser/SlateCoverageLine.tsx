// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {IContest, ISlateCoverage, translate} from "@sequentech/ui-core"
import {theme} from "@sequentech/ui-essentials"

import {ESlateCoverageLabel, getSlateCoverageLabel} from "../../services/SlateCoverage"

export interface ISlateCoverageLineProps {
    coverage: ISlateCoverage
    /** The contests of the ballot the slate has candidates in. */
    contests: Array<IContest>
}

/**
 * What a slate covers: whether it is a full slate and how many candidates it
 * has in how many offices.
 */
export const SlateCoverageLine: React.FC<ISlateCoverageLineProps> = ({coverage, contests}) => {
    const {t, i18n} = useTranslation()
    const {label, contest} = getSlateCoverageLabel(coverage, contests)
    const kind =
        label === ESlateCoverageLabel.FULL
            ? t("slates.coverage.full")
            : label === ESlateCoverageLabel.SINGLE_CONTEST && contest
              ? t("slates.coverage.singleContest", {
                    contest: translate(contest, "name", i18n.language) ?? "",
                })
              : t("slates.coverage.partial")

    return (
        <Box className="slate-coverage" data-coverage={coverage.kind}>
            <Typography
                className="slate-coverage-kind"
                component="span"
                fontSize="14px"
                fontWeight="bold"
            >
                {kind}
            </Typography>
            <Typography
                className="slate-coverage-count"
                component="span"
                fontSize="14px"
                color={theme.palette.customGrey.dark}
            >
                {" · "}
                {t("slates.coverage.candidates", {count: coverage.members})}
                {" · "}
                {t("slates.coverage.offices", {count: coverage.covered.length})}
            </Typography>
        </Box>
    )
}
