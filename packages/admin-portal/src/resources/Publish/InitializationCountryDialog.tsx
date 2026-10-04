// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useQuery} from "@apollo/client"
import {
    Alert,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    MenuItem,
    TextField,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {GET_INITIALIZATION_COUNTRIES} from "@/queries/GetInitializationCountries"
import {
    initializationCountries,
    initializationAreaIds,
    type InitializationCountriesData,
} from "./initializationCountries"

export const InitializationCountryDialog = ({
    electionEventId,
    electionId,
    busy,
    onClose,
    onGenerate,
}: {
    electionEventId: string
    electionId: string
    busy: boolean
    onClose: () => void
    onGenerate: (areaIds?: string[]) => Promise<boolean>
}) => {
    const {t} = useTranslation()
    const [country, setCountry] = useState("")
    const {data, loading, error} = useQuery<InitializationCountriesData>(
        GET_INITIALIZATION_COUNTRIES,
        {
            variables: {electionEventId, electionId},
            fetchPolicy: "network-only",
        }
    )
    const countries = data ? initializationCountries(data, electionId) : []
    const selectionValid = !country || countries.some((area) => area.id === country)
    const generate = async () => {
        if (busy || loading || error || !countries.length || !selectionValid) return
        if (await onGenerate(initializationAreaIds(country, countries))) onClose()
    }
    return (
        <Dialog open onClose={busy ? undefined : onClose} fullWidth maxWidth="sm">
            <DialogTitle>{t("publish.action.generateInitializationReport")}</DialogTitle>
            <DialogContent>
                <Alert severity="info" sx={{mb: 2}}>
                    {t("publish.initialization.countryInfo")}
                </Alert>
                {loading ? (
                    <CircularProgress size={24} />
                ) : error ? (
                    <Alert severity="error">{t("publish.initialization.countriesError")}</Alert>
                ) : !countries.length ? (
                    <Alert severity="error">{t("publish.initialization.noCountries")}</Alert>
                ) : (
                    <TextField
                        select
                        fullWidth
                        label={t("publish.initialization.country")}
                        value={country}
                        onChange={(event) => setCountry(event.target.value)}
                        disabled={busy}
                    >
                        <MenuItem value="">{t("publish.initialization.entirePost")}</MenuItem>
                        {countries.map((area) => (
                            <MenuItem key={area.id} value={area.id}>
                                {area.name || area.id}
                            </MenuItem>
                        ))}
                    </TextField>
                )}
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose} disabled={busy}>
                    {t("common.label.cancel")}
                </Button>
                <Button
                    onClick={generate}
                    disabled={busy || loading || !!error || !countries.length || !selectionValid}
                >
                    {busy ? (
                        <CircularProgress size={16} />
                    ) : (
                        t("publish.action.generateInitializationReport")
                    )}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
