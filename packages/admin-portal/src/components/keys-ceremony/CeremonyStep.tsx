// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CircularProgress,
    Typography,
} from "@mui/material"
import {
    CreateKeysCeremonyMutation,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
} from "@/gql/graphql"
import {
    useGetList,
    useRefresh,
    SimpleForm,
    TextInput,
    Toolbar,
    SaveButton,
    CheckboxGroupInput,
    useGetOne,
    useNotify,
} from "react-admin"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import Button from "@mui/material/Button"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {styled} from "@mui/material/styles"
import {useMutation} from "@apollo/client"
import { useTranslation } from "react-i18next"
import { isNumber } from "lodash"
import { CREATE_KEYS_CEREMONY } from "@/queries/CreateKeysCeremony"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { isNull, Dialog} from "@sequentech/ui-essentials"

const Error = styled(Typography)`
    color:  ${({theme}) => theme.palette.errorColor};
`

const StyledToolbar = styled(Toolbar)`
    flex-direction: row;
    justify-content: space-between;
`

const BackButton = styled(Button)`
    margin-right: auto;
    background-color: ${({theme}) => theme.palette.grey[100]};
    color:  ${({theme}) => theme.palette.brandColor};
`

export interface CeremonyStepProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void
    electionEvent: Sequent_Backend_Election_Event
    goBack: () => void
}

export const CeremonyStep: React.FC<CeremonyStepProps> = ({
    currentCeremony,
    setCurrentCeremony,
    electionEvent,
    goBack,
}) => {
    console.log(`ceremony step with currentCeremony.id=${currentCeremony?.id ?? null}`)
    const {t} = useTranslation()

    return (
        <>
            <StyledToolbar>
                <BackButton
                    color="info"
                    onClick={goBack}
                >
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </BackButton>
            </StyledToolbar>
        </>
    )
}
