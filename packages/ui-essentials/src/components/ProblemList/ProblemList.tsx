// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Alert from "@mui/material/Alert"
import AlertTitle from "@mui/material/AlertTitle"
import Box from "@mui/material/Box"
import Chip from "@mui/material/Chip"
import Link from "@mui/material/Link"
import Stack from "@mui/material/Stack"
import Typography from "@mui/material/Typography"
import {useTranslation} from "react-i18next"

import {problemSentence} from "./sentence"
import {errorsOf, warningsOf, type Problem, type ProblemReport} from "./types"

/**
 * Everything an import or a validation found, in a form someone can act on.
 *
 * The Election Architect and the Admin Portal both show this, and must show it the
 * same way: a delivery engineer who has learned to read it in one should not have
 * to learn it again in the other, and a complaint translated for one is translated
 * for both.
 *
 * Takes a {@link ProblemReport} rather than reaching for the core itself, so it
 * renders in a test with no WASM present and so a caller can filter or merge
 * reports before showing them.
 */
export interface ProblemListProps {
    report: ProblemReport
    /**
     * Shown instead of the list when there is nothing wrong. Silence reads as "it
     * did not run".
     */
    emptyMessage?: string
    /**
     * Treat warnings as blocking, the way a strict build does. Only changes what
     * this says; the caller decides what to do about it.
     */
    strict?: boolean
    /**
     * Take somebody to the thing a problem is about.
     *
     * Given a validation path, the host switches to whatever screen owns it and
     * puts the cursor in the field. Optional because not every host can: an
     * import's paths point into a file the host cannot open, so there the lead
     * stays words rather than pretending to be a link.
     */
    onGoTo?: (path: string) => void
}

export const ProblemList = ({
    report,
    emptyMessage,
    strict = false,
    onGoTo,
}: ProblemListProps): React.JSX.Element => {
    const {t} = useTranslation()
    const errors = errorsOf(report)
    const warnings = warningsOf(report)

    if (errors.length === 0 && warnings.length === 0) {
        return (
            <Alert severity="success">
                {emptyMessage ?? t("problems.noProblems", "Nothing to report. This would import.")}
            </Alert>
        )
    }

    return (
        <Stack spacing={2} data-testid="problem-list">
            {errors.length > 0 && (
                <Group
                    severity="error"
                    title={t("problems.errors", {
                        defaultValue_one: "{{count}} error",
                        defaultValue_other: "{{count}} errors",
                        count: errors.length,
                    })}
                    explanation={t(
                        "problems.errorsExplained",
                        "This will not import until every one of these is fixed."
                    )}
                    problems={errors}
                    onGoTo={onGoTo}
                />
            )}
            {warnings.length > 0 && (
                <Group
                    // Under a strict build a warning stops the build, so showing
                    // it in the colour of something optional would be a lie.
                    severity={strict ? "error" : "warning"}
                    title={t("problems.warnings", {
                        defaultValue_one: "{{count}} warning",
                        defaultValue_other: "{{count}} warnings",
                        count: warnings.length,
                    })}
                    explanation={
                        strict
                            ? t(
                                  "problems.warningsStrict",
                                  "Strict mode is on, so these stop the build."
                              )
                            : t(
                                  "problems.warningsExplained",
                                  "This will import. Each of these is something that is probably not what was meant."
                              )
                    }
                    problems={warnings}
                    onGoTo={onGoTo}
                />
            )}
        </Stack>
    )
}

const Group = ({
    severity,
    title,
    explanation,
    problems,
    onGoTo,
}: {
    severity: "error" | "warning"
    title: string
    explanation: string
    problems: Problem[]
    onGoTo?: (path: string) => void
}): React.JSX.Element => (
    <Alert severity={severity} data-testid={`problem-group-${severity}`}>
        <AlertTitle>{title}</AlertTitle>
        <Typography variant="body2" sx={{mb: 1}}>
            {explanation}
        </Typography>
        <Stack spacing={1.5} component="ul" sx={{m: 0, pl: 2}}>
            {problems.map((problem, index) => (
                <ProblemRow
                    // Two problems can share a code and a path — a duplicated id
                    // is reported once per entity — so the index is part of the
                    // key rather than a lazy stand-in for one.
                    key={`${problem.code}-${problem.path}-${index}`}
                    problem={problem}
                    onGoTo={onGoTo}
                />
            ))}
        </Stack>
    </Alert>
)

/**
 * One problem, as a sentence whose opening words are the way to the field.
 *
 * The `code` is not on the screen — it is a category for grepping, not language —
 * and neither is the `path`, which is the platform talking to itself and a second
 * copy of what the sentence already says. The path is the link's `title` and its
 * test id, so an engineer can still get at it. The `external_id` stays, because
 * that is a name the author typed and it tells four contests apart.
 */
const ProblemRow = ({
    problem,
    onGoTo,
}: {
    problem: Problem
    onGoTo?: (path: string) => void
}): React.JSX.Element => {
    const {t} = useTranslation()
    const {lead, rest} = problemSentence(
        (key, options) => String(t(key, options as never)),
        problem
    )
    return (
        <Box component="li" data-testid="problem">
            <Typography variant="body2">
                {onGoTo === undefined ? (
                    <Box component="span" sx={{fontWeight: 600}}>
                        {lead}
                    </Box>
                ) : (
                    <Link
                        component="button"
                        type="button"
                        variant="body2"
                        data-testid={`goto-${problem.path}`}
                        onClick={() => onGoTo(problem.path)}
                        title={problem.path}
                        sx={{
                            textAlign: "left",
                            fontWeight: 600,
                            verticalAlign: "baseline",
                            // Underlined at rest, not only on hover: this is the
                            // one interactive thing in a panel of prose, and a
                            // link that only announces itself on hover is one a
                            // touch user never discovers.
                            textDecoration: "underline",
                            // The alert's own text colour. The theme's link blue
                            // on an alert's tint falls short of 4.5:1, and the
                            // underline already says "this goes somewhere".
                            color: "inherit",
                        }}
                    >
                        {lead}
                    </Link>
                )}
                {rest}
            </Typography>
            {problem.external_id !== undefined && (
                <Stack
                    direction="row"
                    spacing={1}
                    alignItems="center"
                    sx={{mt: 0.5, flexWrap: "wrap", gap: 0.5}}
                >
                    <Chip size="small" variant="outlined" label={problem.external_id} />
                </Stack>
            )}
        </Box>
    )
}
