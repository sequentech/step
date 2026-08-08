// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {useNavigate} from "react-router-dom"
import {Alert, AlertTitle, Box, Button, Typography} from "@mui/material"
import {PageLimit, theme} from "@sequentech/ui-essentials"
import {translate} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"

import {SettingsContext} from "../providers/SettingsContextProvider"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {
    PreviewDocument,
    updateBallotStyleAndSelection,
} from "./PreviewPublicationEvent"

/**
 * Where a preview handed over as a file gets hydrated, and where it is kept so
 * it survives the reload the auth path does.
 *
 * `sessionStorage` rather than `localStorage`: a preview is a working document
 * for one tab, and leaving somebody's unpublished ballot in a browser after they
 * close it is not something a preview screen should do.
 */
export const PREVIEW_FILE_KEY = "previewFromFile"
export const PREVIEW_FILE_AREA_KEY = "previewFromFileArea"

/** What is wrong with the file, in words somebody can act on. */
const rejects = (document: unknown): string | null => {
    if (typeof document !== "object" || document === null) {
        return "That file does not contain a JSON object."
    }
    const carrier = document as Partial<PreviewDocument>
    if (!Array.isArray(carrier.ballot_styles)) {
        return "That file has no `ballot_styles`, so it is not a ballot preview."
    }
    if (carrier.ballot_styles.length === 0) {
        return "That preview contains no ballots. Nobody would be given one."
    }
    if (typeof carrier.election_event !== "object" || carrier.election_event === null) {
        return "That file has no `election_event`."
    }
    return null
}

/** One entry per area, labelled by what is actually on its ballot. */
interface Choice {
    areaId: string
    label: string
}

const choices = (document: PreviewDocument, language: string): Array<Choice> => {
    const byArea = new Map<string, Array<string>>()
    for (const style of document.ballot_styles) {
        const contests = style.contests ?? []
        const named = contests.map(
            (contest) => translate(contest, "name", language) || contest.id
        )
        byArea.set(style.area_id, [...(byArea.get(style.area_id) ?? []), ...named])
    }
    return [...byArea.entries()].map(([areaId, contests]) => ({
        areaId,
        // The document carries no area names — a ballot style names its area by
        // id — so it is labelled by what is on it, which is the thing somebody
        // previewing actually wants to tell the ballots apart by.
        label: contests.length > 0 ? contests.join(", ") : "an empty ballot",
    }))
}

/**
 * Open a ballot preview from a file.
 *
 * The sibling of [`PreviewPublicationEvent`], which fetches the same document
 * from the public bucket after the Admin Portal has published it there. This one
 * takes the file directly, which is what makes it useful before anything has
 * been imported: `step-cli step compile-plan --preview` writes exactly this
 * document, and so does the Election Architect's review step.
 *
 * The file is read in the browser. Nothing is uploaded, which for an unpublished
 * ballot is the point rather than a detail.
 *
 * It is deliberately not a URL or a `postMessage` listener. This page is
 * unauthenticated and frameable, so a channel that let another origin push a
 * document into it would be new attack surface for a convenience — and a file
 * the operator picks is a document they have already seen.
 */
export const PreviewFromFile: React.FC = () => {
    const {t, i18n} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const navigate = useNavigate()
    const dispatch = useAppDispatch()
    const ballotStyle = useAppSelector(selectFirstBallotStyle)

    const [document_, setDocument] = useState<PreviewDocument | null>(null)
    const [failure, setFailure] = useState<string | null>(null)

    // A preview opened before signing in has to survive `App`'s reload, which is
    // how the authenticated path gets a session. Held in `sessionStorage` and
    // read back here.
    useEffect(() => {
        const held = sessionStorage.getItem(PREVIEW_FILE_KEY)
        if (held === null || document_ !== null) {
            return
        }
        try {
            const parsed = JSON.parse(held) as PreviewDocument
            const area = sessionStorage.getItem(PREVIEW_FILE_AREA_KEY)
            setDocument(parsed)
            if (area !== null) {
                hydrate(parsed, area)
            }
        } catch {
            sessionStorage.removeItem(PREVIEW_FILE_KEY)
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    useEffect(() => {
        if (ballotStyle?.election_event_id && ballotStyle.tenant_id) {
            navigate(
                `/tenant/${ballotStyle.tenant_id}/event/${ballotStyle.election_event_id}/election-chooser`
            )
        }
    }, [ballotStyle?.election_event_id, ballotStyle?.tenant_id, navigate])

    const read = async (file: File): Promise<void> => {
        try {
            const parsed = JSON.parse(await file.text()) as unknown
            const wrong = rejects(parsed)
            if (wrong !== null) {
                setFailure(wrong)
                setDocument(null)
                return
            }
            setFailure(null)
            setDocument(parsed as PreviewDocument)
        } catch (error) {
            setDocument(null)
            setFailure(
                error instanceof Error ? error.message : String(error)
            )
        }
    }

    const hydrate = (preview: PreviewDocument, areaId: string): void => {
        const tenantId = preview.ballot_styles[0]?.tenant_id ?? ""
        try {
            sessionStorage.setItem("isDemo", "true")
            sessionStorage.setItem(PREVIEW_FILE_AREA_KEY, areaId)
            sessionStorage.setItem(PREVIEW_FILE_KEY, JSON.stringify(preview))
        } catch {
            // Over the quota. Worth saying rather than failing later at the
            // reload, where it would look like the preview simply did not work.
            if (!globalSettings.DISABLE_AUTH) {
                setFailure(
                    "That preview is too large to keep across signing in. It " +
                        "will open, but if you are asked to sign in you will " +
                        "have to choose the file again."
                )
            }
        }
        try {
            updateBallotStyleAndSelection(
                preview,
                tenantId,
                areaId,
                dispatch
            )
        } catch (error) {
            setFailure(
                `That ballot could not be loaded: ${
                    error instanceof Error ? error.message : String(error)
                }`
            )
        }
    }

    return (
        <PageLimit maxWidth="md">
            <Box sx={{display: "flex", flexDirection: "column", gap: 2, py: 4}}>
                <Typography variant="h4">
                    {t("previewFromFile.title", "Preview a ballot")}
                </Typography>
                <Typography variant="body1" color={theme.palette.customGrey.contrastText}>
                    {t(
                        "previewFromFile.blurb",
                        "Open a ballot preview file to see the ballot exactly as voters will. The file is read here in your browser — nothing is uploaded — and the ballots it contains cannot be voted on."
                    )}
                </Typography>

                <Box>
                    <Button variant="contained" component="label">
                        {t("previewFromFile.choose", "Choose a preview file")}
                        <input
                            hidden
                            type="file"
                            accept="application/json,.json"
                            data-testid="preview-file"
                            onChange={(event) => {
                                const file = event.target.files?.[0]
                                if (file !== undefined) {
                                    void read(file)
                                }
                                event.target.value = ""
                            }}
                        />
                    </Button>
                </Box>

                {failure !== null && (
                    <Alert severity="error" data-testid="preview-file-failed">
                        {failure}
                    </Alert>
                )}

                {document_ !== null && (
                    <Box sx={{display: "flex", flexDirection: "column", gap: 1}}>
                        <Alert severity="info">
                            <AlertTitle>
                                {t(
                                    "previewFromFile.notReal",
                                    "This is a preview, not an election"
                                )}
                            </AlertTitle>
                            {t(
                                "previewFromFile.notRealHelp",
                                "The key it carries is a stand-in, so nothing you do here is recorded and no vote can be cast."
                            )}
                        </Alert>
                        <Typography variant="h6">
                            {t("previewFromFile.pick", "Which ballot?")}
                        </Typography>
                        {choices(document_, i18n.language).map((choice) => (
                            <Button
                                key={choice.areaId}
                                variant="outlined"
                                sx={{justifyContent: "flex-start"}}
                                data-testid={`preview-area-${choice.areaId}`}
                                onClick={() => hydrate(document_, choice.areaId)}
                            >
                                {choice.label}
                            </Button>
                        ))}
                    </Box>
                )}
            </Box>
        </PageLimit>
    )
}

export default PreviewFromFile
