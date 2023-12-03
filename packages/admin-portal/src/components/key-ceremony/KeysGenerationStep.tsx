// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    CircularProgress,
    Typography,
} from "@mui/material"
import {
    CreateKeyCeremonyMutation,
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
import { CREATE_KEY_CEREMONY } from "@/queries/CreateKeyCeremony"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { isNull } from "@sequentech/ui-essentials"

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
const CreateButton = styled(SaveButton)`
    margin-left: auto;
    flex-direction: row-reverse;
`

export interface KeysGenerationStepProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keyCeremony: Sequent_Backend_Keys_Ceremony) => void
    electionEvent: Sequent_Backend_Election_Event
    goBack: () => void
}

export const KeysGenerationStep: React.FC<KeysGenerationStepProps> = ({
    currentCeremony: currentCeremony,
    setCurrentCeremony,
    electionEvent,
    goBack,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const [createKeyCeremonyMutation] = useMutation<CreateKeyCeremonyMutation>(CREATE_KEY_CEREMONY)
    const [errors, setErrors] = useState<String | null>(null)
    const [threshold, __] = useState<number>(2)
    const refresh = useRefresh()
    const {data: trusteeList, total, isLoading: _, error} = useGetList(
        "sequent_backend_trustee",
        {
            pagination: {page: 1, perPage: 10},
            sort: {field: "last_updated_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
            },
        }
    )
    const {
        data: keyCeremony,
        isLoading: isOneLoading,
    } = useGetOne<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            id: newId,
            meta: {
                tenant_id: tenantId,
                election_event_id: electionEvent.id,
            }
        }
    )

    useEffect(() => {
        if (isNull(newId)) {
            return
        }
        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            notify(
                t(
                    "keysGenerationStep.errorCreatingCeremony",
                    {code: error + ""}
                ),
                {type: "error"}
            )
            refresh()
            return
        }
        if (isLoading && !error && !isOneLoading && !currentCeremony && keyCeremony) {
            setIsLoading(false)
            setCurrentCeremony(keyCeremony)
            notify(
                t("keysGenerationStep.createCeremonySuccess"),
                {type: "success"}
            )
            refresh()
            return
        }
    }, [isLoading, keyCeremony, isOneLoading, error])

    if (isLoading || error) {
        return null
    }

    const createKeyCeremony:
        (input: {threshold: number, trusteeList: string[]}) => Promise<string | null> =
        async ({threshold, trusteeList}) =>
    {
        const {data, errors} = await createKeyCeremonyMutation({
            variables: {
                electionEventId: electionEvent.id,
                threshold: threshold,
                trusteeNames: trusteeList,
            },
        })
        if (errors) {
            setErrors(t(
                "keysGenerationStep.errorCreatingCeremony",
                {code: error + ""}
            ))
            return null
        }
        if (data) {
            console.log(data)
        }
        return data?.create_key_ceremony?.key_ceremony_id ?? null
    }

    const onSubmit: SubmitHandler<FieldValues> = async ({
        threshold,
        trusteeList,

    }) => {
        if (isLoading) {
            return
        }
        setErrors(null)
        setIsLoading(true)
        try {
            const keyCeremonyId = await createKeyCeremony({
                threshold, trusteeList
            })
            if (keyCeremonyId) {
                setNewId(keyCeremonyId)
            } else {
                notify(t("tenantScreen.createError"), {type: "error"})
                setIsLoading(false)
            }
        } catch (error) {
            setErrors(t(
                "keysGenerationStep.errorCreatingCeremony",
                {code: error + ""}
            ))
            setIsLoading(false)
        }
    }

    const getDefaultValues = () => ({
        threshold: 2
    })

    const thresholdValidator = (value: any): any => {
        const max = (trusteeList) ? trusteeList.length : 0
        var intValue: number | null = null
        try {
            intValue = parseInt(value)
        } catch {
            intValue = null
        }
        if (
            !isNumber(intValue) ||
            isNaN(intValue) ||
            intValue < 2 ||
            intValue > max
        ) {
            return t(
                "keysGenerationStep.errorThreshold",
                {selected: intValue, min: 0, max: max}
            )
        }
    }

    const trusteeListValidator = (values: any): any => {
        const length = (values && values.length) ? values.length : 0
        if (length < threshold) {
            return t(
                "keysGenerationStep.errorMinTrustees",
                {selected: length, threshold: threshold}
            )
        }
    }

    return (
        <SimpleForm
            defaultValues={getDefaultValues}
            onSubmit={onSubmit}
            toolbar={
                <StyledToolbar>
                    <BackButton
                        color="info"
                        onClick={goBack}
                    >
                        <ArrowBackIosIcon />
                        {t("common.label.back")}
                    </BackButton>
                    <CreateButton
                        icon={<ArrowForwardIosIcon />}
                        label={t("keysGenerationStep.create")}
                    />
                </StyledToolbar>
            }
        >
            <Typography variant="h4">
                {t("keysGenerationStep.title")}
            </Typography>
            <Typography variant="body2">
                {t("keysGenerationStep.subtitle")}
            </Typography>

            <TextInput
                source="threshold"
                label={t("keysGenerationStep.threshold")}
                value={threshold}
                validate={thresholdValidator}
                type="number"
                InputLabelProps={{
                    shrink: true,
                }}
                variant="filled"
            />
            {trusteeList ? (
                <CheckboxGroupInput
                    validate={trusteeListValidator}
                    label={t("keysGenerationStep.trusteeList")}
                    source="selectedTrustees"
                    choices={trusteeList}
                    translateChoice={false}
                    optionText="name"
                    optionValue="id"
                    row={false}
                />
            ) : null}
            {errors ? <Error variant="body2">{errors}</Error> : null}
            {isLoading ? <CircularProgress /> : null}
        </SimpleForm>
    )
}
