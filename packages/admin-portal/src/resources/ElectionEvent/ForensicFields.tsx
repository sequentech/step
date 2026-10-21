// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, ButtonBase, Chip, Stack, Typography} from "@mui/material"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {faUserSecret} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import {COUNTRIES} from "@/lib/countries"

export const ForensicPseudonym: React.FC<{code: string}> = ({code}) => (
    <Chip
        icon={<FontAwesomeIcon icon={faUserSecret} />}
        label={code}
        size="small"
        variant="outlined"
        sx={{
            "height": 24,
            "borderRadius": "4px",
            "bgcolor": "#F7F9FC",
            "borderColor": "#DFE6EF",
            "color": "#405269",
            "& .MuiChip-label": {
                fontFamily: '"DejaVu Sans Mono", ui-monospace, monospace',
                fontSize: 12,
                fontWeight: 600,
                letterSpacing: 0.5,
            },
            "& .MuiChip-icon": {color: "#64748B", fontSize: 11, ml: 1},
        }}
    />
)

export const ForensicCountry: React.FC<{code: string | null; showName?: boolean}> = ({
    code,
    showName,
}) => (
    <Stack component="span" direction="row" alignItems="center" spacing={1}>
        <Box
            component="span"
            sx={{
                fontSize: 11,
                fontWeight: 600,
                color: "#475569",
                border: "1px solid #CBD5E1",
                borderRadius: "4px",
                px: 0.75,
                py: 0.15,
                lineHeight: 1.5,
            }}
        >
            {code ?? "—"}
        </Box>
        {showName && code ? (
            <Typography component="span" variant="body2" color="text.secondary">
                {COUNTRIES.find((country) => country.code === code)?.name}
            </Typography>
        ) : null}
    </Stack>
)

export const ForensicBallotCount: React.FC<{count: number; shared: boolean}> = ({
    count,
    shared,
}) => {
    const {t} = useTranslation()
    return (
        <Stack
            component="span"
            direction="row"
            alignItems="center"
            justifyContent="flex-end"
            spacing={1}
            sx={{whiteSpace: "nowrap"}}
        >
            {shared ? (
                <Chip
                    label={t("ballotPrivacy.forensic.shared")}
                    title={t("ballotPrivacy.forensic.sharedHelp")}
                    size="small"
                    sx={{
                        height: 22,
                        borderRadius: "100px",
                        fontSize: 12,
                        fontWeight: 500,
                        bgcolor: "#FFF7D9",
                        color: "#6B5B26",
                        border: "1px solid #F1E3A6",
                    }}
                />
            ) : null}
            <Box component="span" sx={{fontWeight: 600, fontVariantNumeric: "tabular-nums"}}>
                {count}
            </Box>
        </Stack>
    )
}

export const ForensicAddressButton: React.FC<{ip: string; onOpen: () => void}> = ({ip, onOpen}) => (
    <ButtonBase
        onClick={onOpen}
        sx={{
            "fontFamily": '"DejaVu Sans Mono", ui-monospace, monospace',
            "fontSize": 14,
            "color": "text.primary",
            "textAlign": "start",
            "overflowWrap": "anywhere",
            "justifyContent": "flex-start",
            "minWidth": 0,
            "borderRadius": "2px",
            "&:hover": {textDecoration: "underline"},
            "&.Mui-focusVisible": {
                outline: "2px solid",
                outlineColor: "primary.main",
                outlineOffset: 3,
            },
        }}
    >
        {ip}
    </ButtonBase>
)
