// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {CircularProgress, Typography, TextField, InputLabel, Select, MenuItem} from "@mui/material"
import {
    CreateKeysCeremonyMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Trustee,
} from "@/gql/graphql"
import {
    useGetList,
    useRefresh,
    SimpleForm,
    TextInput,
    CheckboxGroupInput,
    useGetOne,
    useNotify,
    ValidationErrorMessage,
} from "react-admin"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CREATE_KEYS_CEREMONY} from "@/queries/CreateKeysCeremony"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Dialog} from "@sequentech/ui-essentials"
import {isNull} from "@sequentech/ui-core"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {IPermissions} from "@/types/keycloak"

const ALL_ELECTIONS = "all-elections"

export interface ConfigureStepProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony | null
    setCurrentCeremony: (keysCeremony: Sequent_Backend_Keys_Ceremony) => void
    electionEvent: Sequent_Backend_Election_Event
    openCeremonyStep: () => void
    goBack: () => void
}

export const ConfigureStep: React.FC<ConfigureStepProps> = ({
    currentCeremony,
    setCurrentCeremony,
    electionEvent,
    openCeremonyStep,
    goBack,
}) => {
    const {t, i18n} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [newId, setNewId] = useState<string | null>(null)
    const [isLoading, setIsLoading] = useState(false)
    const [openConfirmationModal, setOpenConfirmationModal] = useState(false)
    const [createKeysCeremonyMutation] = useMutation<CreateKeysCeremonyMutation>(
        CREATE_KEYS_CEREMONY,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.ADMIN_CEREMONY,
                },
            },
        }
    )
    const [errors, setErrors] = useState<String | null>(null)
    const [threshold, setThreshold] = useState<number>(2)
    const [name, setName] = useState<string>("")
    const [electionId, setElectionId] = useState<string | null>(ALL_ELECTIONS)
    const [trusteeNames, setTrusteeNames] = useState<string[]>([])
    const refresh = useRefresh()
    const aliasRenderer = useAliasRenderer()
    const {data: trusteeList, error} = useGetList<Sequent_Backend_Trustee>(
        "sequent_backend_trustee",
        {
            pagination: {page: 1, perPage: 10},
            sort: {field: "last_updated_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
            },
        }
    )
    const {data: electionsList} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            tenant_id: electionEvent.tenant_id,
            election_event_id: electionEvent.id,
            keys_ceremony_id: {
                format: "hasura-raw-query",
                value: {_is_null: true},
            },
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
        electionId?: string
        name?: string
    }) => Promise<string | null> = async ({threshold, trusteeNames, electionId, name}) => {
        const {data, errors} = await createKeysCeremonyMutation({
            variables: {
                electionEventId: electionEvent.id,
                threshold,
                trusteeNames,
                electionId: (ALL_ELECTIONS !== electionId && electionId) || null,
                name: name ?? t("keysGeneration.configureStep.name"),
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
        let election = electionsList?.find((election) => election.id === electionId)
        let electionName = electionId
            ? election
                ? aliasRenderer(election)
                : ""
            : t("keysGeneration.configureStep.allElections")

        try {
            const keysCeremonyId = await createKeysCeremony({
                threshold,
                trusteeNames,
                name: electionName,
                electionId: electionId ?? undefined,
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
        setThreshold(Number(threshold))
        setTrusteeNames(trusteeNames)
        setOpenConfirmationModal(true)
    }

    // Default values
    const getDefaultValues = () => ({
        threshold: 2,
    })

    // validates threshold is within the limits
    const thresholdValidator = (value: string): ValidationErrorMessage | null => {
        const thresholdInput = Number(value)
        const max = trusteeList?.length ?? 0

        if (thresholdInput < 2 || thresholdInput > max) {
            return t("keysGeneration.configureStep.errorThreshold", {
                selected: thresholdInput,
                min: 2,
                max: max,
            })
        }

        return null
    }

    // validates selected trustees
    const trusteeListValidator = (value: string[]): ValidationErrorMessage | null => {
        const length = value && value ? value.length : 0
        if (length < threshold) {
            return t("keysGeneration.configureStep.errorMinTrustees", {
                selected: length,
                threshold: threshold,
                count: length,
            })
        } else {
            return null
        }
    }

    const validateTrusteeList = [trusteeListValidator]
    const validateThreshold = [thresholdValidator]
    const onElectionChange = (id: string | null) => {
        setElectionId(id)
    }

    return (
        <>
            <WizardStyles.ContentBox>
                <SimpleForm
                    defaultValues={getDefaultValues}
                    onSubmit={onSubmit}
                    toolbar={
                        <WizardStyles.Toolbar>
                            <WizardStyles.BackButton
                                color="info"
                                onClick={goBack}
                                className="keys-back-button"
                            >
                                <ArrowBackIosIcon />
                                {t("common.label.back")}
                            </WizardStyles.BackButton>
                            <WizardStyles.CreateButton
                                className="keys-create-button"
                                icon={<ArrowForwardIosIcon />}
                                label={t("keysGeneration.configureStep.create")}
                            />
                        </WizardStyles.Toolbar>
                    }
                >
                    <WizardStyles.MainContent>
                        <WizardStyles.StepHeader variant="h4" dir={i18n.dir(i18n.language)}>
                            {t("keysGeneration.configureStep.title")}
                        </WizardStyles.StepHeader>
                        <Typography variant="body2" dir={i18n.dir(i18n.language)}>
                            {t("keysGeneration.configureStep.subtitle")}
                        </Typography>

                        <TextInput
                            dir={i18n.dir(i18n.language)}
                            source="threshold"
                            label={t("keysGeneration.configureStep.threshold")}
                            value={threshold}
                            validate={validateThreshold}
                            type="number"
                            InputLabelProps={{
                                shrink: true,
                            }}
                            variant="filled"
                        />
                        {trusteeList ? (
                            <CheckboxGroupInput
                                dir={i18n.dir(i18n.language)}
                                validate={validateTrusteeList}
                                label={t("keysGeneration.configureStep.trusteeList")}
                                source="trusteeNames"
                                choices={trusteeList}
                                translateChoice={false}
                                optionText="name"
                                optionValue="name"
                                row={false}
                                className="keys-trustees-input"
                            />
                        ) : null}
                        <InputLabel dir={i18n.dir(i18n.language)}>
                            {t("electionScreen.common.title")}
                        </InputLabel>
                        <Select
                            dir={i18n.dir(i18n.language)}
                            value={electionId}
                            label={t("electionScreen.common.title")}
                            onChange={(e) => onElectionChange(e.target.value ?? null)}
                        >
                            <MenuItem value={ALL_ELECTIONS} dir={i18n.dir(i18n.language)}>
                                {t("keysGeneration.configureStep.allElections")}
                            </MenuItem>
                            {electionsList?.map((election) => (
                                <MenuItem
                                    key={election.id}
                                    value={election.id}
                                    dir={i18n.dir(i18n.language)}
                                >
                                    {aliasRenderer(election)}
                                </MenuItem>
                            ))}
                        </Select>
                        {errors ? (
                            <WizardStyles.ErrorMessage variant="body2" className="keys-error">
                                {errors}
                            </WizardStyles.ErrorMessage>
                        ) : null}
                        {isLoading ? <CircularProgress /> : null}
                    </WizardStyles.MainContent>
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
            </WizardStyles.ContentBox>
        </>
    )
}
