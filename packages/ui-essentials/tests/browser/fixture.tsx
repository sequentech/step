// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {createRoot} from "react-dom/client"
import {ThemeProvider} from "@mui/material/styles"
import {I18nextProvider} from "react-i18next"
import i18next from "i18next"
import theme from "../../src/services/theme"
import CustomDropFile from "../../src/components/CustomDropFile/CustomDropFile"
import CandidatesList from "../../src/components/CandidatesList/CandidatesList"
import {useSelectElectionCountdown} from "../../src/components/SelectElection/useSelectElectionCountdown"

function Fixture() {
    const [selected, setSelected] = useState(false)
    const [selectionCalls, setSelectionCalls] = useState(0)
    const [filename, setFilename] = useState("No file")
    const [deadline, setDeadline] = useState("")
    const remaining = useSelectElectionCountdown({date: deadline})
    return (
        <main>
            <h1>UI Essentials browser integration</h1>
            <CustomDropFile
                accept=".json"
                errorMessage="Cannot import this file"
                handleFiles={async (files) => {
                    const file = files[0]
                    JSON.parse(await file.text()) // Exercise a real asynchronous parser failure.
                    setFilename(file.name)
                }}
            >
                Choose election file
            </CustomDropFile>
            <output aria-label="Imported file">{filename}</output>
            <CandidatesList
                title="Council"
                isActive
                isCheckable
                checked={selected}
                setChecked={(value) => {
                    setSelected(value)
                    setSelectionCalls((count) => count + 1)
                }}
                isCollapsible
                defaultExpanded={false}
                collapseToggleAriaLabel="Toggle council"
            >
                <li>Candidate</li>
            </CandidatesList>
            <output aria-label="Selection calls">{selectionCalls}</output>
            <button onClick={() => setDeadline(new Date(Date.now() + 2_000).toISOString())}>
                Start countdown
            </button>
            <output aria-label="Seconds remaining">
                {remaining?.totalSeconds ?? "No countdown"}
            </output>
        </main>
    )
}

await i18next.init({lng: "en", resources: {en: {translation: {a11y: {selectList: "Select list"}}}}})
const root = document.getElementById("root")
if (!root) throw new Error("missing browser fixture root")
createRoot(root).render(
    <I18nextProvider i18n={i18next}>
        <ThemeProvider theme={theme}>
            <Fixture />
        </ThemeProvider>
    </I18nextProvider>
)
