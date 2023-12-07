// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {CircularProgress, Typography} from "@mui/material"
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
import {useTranslation} from "react-i18next"
import {isNumber} from "lodash"
import {CREATE_KEYS_CEREMONY} from "@/queries/CreateKeysCeremony"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {isNull, Dialog} from "@sequentech/ui-essentials"

const Error = styled(Typography)`
    color: ${({theme}) => theme.palette.errorColor};
`

const StyledToolbar = styled(Toolbar)`
    flex-direction: row;
    justify-content: space-between;
`

const BackButton = styled(Button)`
    margin-right: auto;
    background-color: ${({theme}) => theme.palette.grey[100]};
    color: ${({theme}) => theme.palette.brandColor};
`
const CreateButton = styled(SaveButton)`
    margin-left: auto;
    flex-direction: row-reverse;
`

export interface ConfigureStepProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void
    electionEvent: Sequent_Backend_Election_Event
    openCeremonyStep: () => void
    goBack: () => void
}

export const StartStep: React.FC<ConfigureStepProps> = ({
    currentCeremony,
    setCurrentCeremony,
    electionEvent,
    openCeremonyStep,
    goBack,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const [openConfirmationModal, setOpenConfirmationModal] = useState(false)
    const [createKeysCeremonyMutation] =
        useMutation<CreateKeysCeremonyMutation>(CREATE_KEYS_CEREMONY)
    const [errors, setErrors] = useState<String | null>(null)
    const [threshold, setThreshold] = useState<number>(2)
    const [trusteeNames, setTrusteeNames] = useState<string[]>([])
    const refresh = useRefresh()
    const {
        data: trusteeList,
        total,
        isLoading: _,
        error,
    } = useGetList("sequent_backend_trustee", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            tenant_id: electionEvent.tenant_id,
        },
    })
    const {data: keysCeremony, isLoading: isOneLoading} = useGetOne<Sequent_Backend_Keys_Ceremony>(
        "sequent_backend_keys_ceremony",
        {
            id: newId,
            meta: {
                tenant_id: tenantId,
                election_event_id: electionEvent.id,
            },
        }
    )

    useEffect(() => {
        if (isNull(newId)) {
            return
        }
        if (isLoading && error && !isOneLoading) {
            setIsLoading(false)
            notify(t("keysGeneration.configureStep.errorCreatingCeremony", {error: error + ""}), {
                type: "error",
            })
            refresh()
            return
        }
        if (isLoading && !error && !isOneLoading && !currentCeremony && keysCeremony) {
            setIsLoading(false)
            setCurrentCeremony(keysCeremony)
            openCeremonyStep()
            notify(t("keysGeneration.configureStep.createCeremonySuccess"), {type: "success"})
            refresh()
            return
        }
    }, [isLoading, keysCeremony, isOneLoading, error])

    if (isLoading || error) {
        return null
    }

    // called by confirmCreateKeysCeremony() to create the Keys Ceremony
    const createKeysCeremony: (input: {
        threshold: number
        trusteeNames: string[]
    }) => Promise<string | null> = async ({threshold, trusteeNames}) => {
        const {data, errors} = await createKeysCeremonyMutation({
            variables: {
                electionEventId: electionEvent.id,
                threshold: threshold,
                trusteeNames: trusteeNames,
            },
        })
        if (errors) {
            setErrors(t("keysGeneration.configureStep.errorCreatingCeremony", {code: error + ""}))
            return null
        }
        if (data) {
            console.log(data)
        }
        return data?.create_keys_ceremony?.keys_ceremony_id ?? null
    }

    // Called by the confirmation dialog to create the Keys Ceremony
    const confirmCreateKeysCeremony = async () => {
        if (isLoading) {
            return
        }
        setErrors(null)
        setIsLoading(true)
        try {
            const keysCeremonyId = await createKeysCeremony({
                threshold,
                trusteeNames,
            })
            if (keysCeremonyId) {
                setNewId(keysCeremonyId)
            } else {
                notify(t("keysGeneration.configureStep.errorCreatingCeremony", {error: "error"}), {
                    type: "error",
                })
                setIsLoading(false)
            }
        } catch (error) {
            setErrors(t("keysGeneration.configureStep.errorCreatingCeremony", {error: error + ""}))
            setIsLoading(false)
        }
    }

    // Called by the form. Saves the information and shows the confirmation
    // dialog
    const onSubmit: SubmitHandler<FieldValues> = async ({threshold, trusteeNames}) => {
        setThreshold(threshold)
        setTrusteeNames(trusteeNames)
        setOpenConfirmationModal(true)
    }

    // Default values
    const getDefaultValues = () => ({
        threshold: 2,
    })

    // validates threshold is within the limits
    const thresholdValidator = (value: any): any => {
        const max = trusteeList ? trusteeList.length : 0
        var intValue: number | null = null
        try {
            intValue = parseInt(value)
        } catch {
            intValue = null
        }
        if (!isNumber(intValue) || isNaN(intValue) || intValue < 2 || intValue > max) {
            return t("keysGeneration.configureStep.errorThreshold", {
                selected: intValue,
                min: 0,
                max: max,
            })
        }
    }

    // validates selected trustees
    const trusteeListValidator = (values: any): any => {
        const length = values && values.length ? values.length : 0
        if (length < threshold) {
            return t("keysGeneration.configureStep.errorMinTrustees", {
                selected: length,
                threshold: threshold,
            })
        }
    }

    return (
        <>
            <SimpleForm
                defaultValues={getDefaultValues}
                onSubmit={onSubmit}
                toolbar={
                    <StyledToolbar>
                        <BackButton color="info" onClick={goBack}>
                            <ArrowBackIosIcon />
                            {t("common.label.back")}
                        </BackButton>
                        <CreateButton
                            icon={<ArrowForwardIosIcon />}
                            label={t("keysGeneration.configureStep.create")}
                        />
                    </StyledToolbar>
                }
            >
                <Typography variant="h4">{t("keysGeneration.configureStep.title")}</Typography>
                <Typography variant="body2">
                    {t("keysGeneration.configureStep.subtitle")}
                </Typography>

                <TextInput
                    source="threshold"
                    label={t("keysGeneration.configureStep.threshold")}
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
                        label={t("keysGeneration.configureStep.trusteeList")}
                        source="trusteeNames"
                        choices={trusteeList}
                        translateChoice={false}
                        optionText="name"
                        optionValue="name"
                        row={false}
                    />
                ) : null}
                {errors ? <Error variant="body2">{errors}</Error> : null}
                {isLoading ? <CircularProgress /> : null}
            </SimpleForm>
            <Dialog
                variant="warning"
                open={openConfirmationModal}
                ok={t("keysGeneration.configureStep.confirmdDialog.ok")}
                cancel={t("keysGeneration.configureStep.confirmdDialog.cancel")}
                title={t("keysGeneration.configureStep.confirmdDialog.title")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmCreateKeysCeremony()
                    }
                    setOpenConfirmationModal(false)
                }}
            >
                {t("keysGeneration.configureStep.confirmdDialog.description")}
            </Dialog>
        </>
    )
}
