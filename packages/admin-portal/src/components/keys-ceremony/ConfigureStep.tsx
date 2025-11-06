// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {
    CircularProgress,
    Typography,
    InputLabel,
    Select,
    MenuItem,
    FormControl,
    OutlinedInput,
    IconButton,
    Autocomplete,
    TextField,
} from "@mui/material"
import InputAdornment from "@mui/material/InputAdornment"
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
    AutocompleteInput,
    ReferenceInput,
    BooleanInput,
} from "react-admin"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CREATE_KEYS_CEREMONY} from "@/queries/CreateKeysCeremony"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Dialog} from "@sequentech/ui-essentials"
import {
    EElectionEventCeremoniesPolicy,
    IElectionEventPresentation,
    isNull,
} from "@sequentech/ui-core"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {IPermissions} from "@/types/keycloak"
import {Clear} from "@mui/icons-material"
import {CreateKeysError} from "@/types/ceremonies"

const ITEM_HEIGHT = 48
const ITEM_PADDING_TOP = 8
const MenuProps = {
    PaperProps: {
        style: {
            maxHeight: ITEM_HEIGHT * 4 + ITEM_PADDING_TOP,
        },
    },
}
const TRUSTEE_CHECKBOXES_SX = {
    [`.MuiFormGroup-root`]: {
        width: "100%",
        height: "200px",
        display: "flex",
        flexDirection: "column",
        flexFlow: "column",
        overflowY: "scroll",
    },
}

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
    const [isAutomaticCeremony, setIsAutomaticCeremony] = useState<boolean>(false)
    const [electionId, setElectionId] = useState<string | null>(null)
    const [trusteeNames, setTrusteeNames] = useState<string[]>([])
    const refresh = useRefresh()
    const aliasRenderer = useAliasRenderer()
    const {data: trusteeList, error} = useGetList<Sequent_Backend_Trustee>(
        "sequent_backend_trustee",
        {
            pagination: {page: 1, perPage: 200},
            sort: {field: "last_updated_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
            },
        }
    )
    const {data: electionById} = useGetOne<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            id: electionId,
            meta: {
                tenant_id: tenantId,
                election_event_id: electionEvent.id,
                keys_ceremony_id: {
                    format: "hasura-raw-query",
                    value: {_is_null: true},
                },
            },
        },
        {enabled: !!electionId}
    )
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

    const [filterTrustees, setFilterTrustees] = useState<string>("")
    const [filteredTrustees, setFilteredTrustees] = useState<
        Sequent_Backend_Trustee[] | undefined
    >()

    const filteredTrusteesSorted = useMemo(
        () =>
            [...(filteredTrustees ?? [])]?.sort((a, b) =>
                (a.name ?? "").localeCompare(b.name ?? "")
            ),
        [filteredTrustees]
    )
    const trusteeListSorted = useMemo(
        () => [...(trusteeList ?? [])]?.sort((a, b) => (a.name ?? "").localeCompare(b.name ?? "")),
        [trusteeList]
    )

    useEffect(() => {
        setFilteredTrustees(
            trusteeList?.filter((trustee: Sequent_Backend_Trustee) =>
                trustee?.name?.toLowerCase().includes(filterTrustees)
            ) ?? []
        )
    }, [filterTrustees, trusteeList])

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
        isAutomaticCeremony: boolean
    }) => Promise<string | null> = async ({
        threshold,
        trusteeNames,
        electionId,
        name,
        isAutomaticCeremony,
    }) => {
        const {data, errors} = await createKeysCeremonyMutation({
            variables: {
                electionEventId: electionEvent.id,
                threshold,
                trusteeNames,
                electionId: electionId || null,
                name: name ?? t("keysGeneration.configureStep.name"),
                isAutomaticCeremony: isAutomaticCeremony ?? false,
            },
        })

        let error_message = data?.create_keys_ceremony?.error_message
        if (error_message) {
            let error = error_message as CreateKeysError
            if (error == CreateKeysError.PERMISSION_LABELS) {
                setErrors(t("keysGeneration.configureStep.errorPermisionLabels"))
            }
        }
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
        let electionName = electionId
            ? electionById
                ? aliasRenderer(electionById)
                : t("keysGeneration.configureStep.allElections")
            : t("keysGeneration.configureStep.allElections")

        try {
            const keysCeremonyId = await createKeysCeremony({
                threshold,
                trusteeNames,
                name: electionName,
                electionId: electionId ?? undefined,
                isAutomaticCeremony: isAutomaticCeremony,
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
    const onSubmit: SubmitHandler<FieldValues> = async ({
        threshold,
        trusteeNames,
        electionId,
        isAutomatic,
    }) => {
        setThreshold(Number(threshold))
        setTrusteeNames(trusteeNames)
        setOpenConfirmationModal(true)
        setElectionId(electionId ?? null)
        setIsAutomaticCeremony(isAutomatic)
    }

    // Default values
    const getDefaultValues = () => ({
        threshold: 2,
        electionId: null,
    })

    const electionFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {"name@_ilike,alias@_ilike": searchText.trim()}
    }

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

    const isElectionEventAutomatedCeremonyPolicy =
        electionEvent.presentation?.ceremonies_policy ===
        EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES

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
                        {isElectionEventAutomatedCeremonyPolicy && (
                            <BooleanInput
                                disabled={!isElectionEventAutomatedCeremonyPolicy}
                                source="isAutomatic"
                                label={t("keysGeneration.configureStep.automaticCeremonyToggle")}
                            />
                        )}
                        {trusteeList ? (
                            <>
                                <InputLabel dir={i18n.dir(i18n.language)}>
                                    {t("keysGeneration.configureStep.trusteeList")}
                                </InputLabel>
                                <FormControl>
                                    <InputLabel htmlFor="trustees-filter">
                                        {t("keysGeneration.configureStep.filterTrustees")}
                                    </InputLabel>
                                    <OutlinedInput
                                        id="trustees-filter"
                                        dir={i18n.dir(i18n.language)}
                                        label={t("keysGeneration.configureStep.filterTrustees")}
                                        value={filterTrustees}
                                        type="text"
                                        onChange={(e) => setFilterTrustees(e.target.value)}
                                        endAdornment={
                                            <InputAdornment position="end">
                                                <IconButton
                                                    onClick={() => setFilterTrustees("")}
                                                    edge="end"
                                                >
                                                    <Clear />
                                                </IconButton>
                                            </InputAdornment>
                                        }
                                    />
                                </FormControl>
                                <CheckboxGroupInput
                                    sx={TRUSTEE_CHECKBOXES_SX}
                                    dir={i18n.dir(i18n.language)}
                                    validate={validateTrusteeList}
                                    label=""
                                    source="trusteeNames"
                                    choices={filteredTrusteesSorted || trusteeListSorted}
                                    translateChoice={false}
                                    optionText="name"
                                    optionValue="name"
                                    row={false}
                                    className="keys-trustees-input"
                                />
                            </>
                        ) : null}
                        <InputLabel dir={i18n.dir(i18n.language)}>
                            {t("electionScreen.common.title")}
                        </InputLabel>

                        <ReferenceInput
                            fullWidth
                            label={t("electionScreen.common.title")}
                            source="electionId"
                            id="searchable-elections"
                            reference="sequent_backend_election"
                            enableGetChoices={({q}) => q && q.length >= 3}
                            filter={{
                                tenant_id: electionEvent.tenant_id,
                                election_event_id: electionEvent.id,
                                keys_ceremony_id: {
                                    format: "hasura-raw-query",
                                    value: {_is_null: true},
                                },
                            }}
                            perPage={50}
                            sort={{field: "alias", order: "ASC"}}
                        >
                            <AutocompleteInput
                                className="election-selector"
                                optionText={aliasRenderer}
                                filterToQuery={electionFilterToQuery}
                                debounce={100}
                                emptyText={t("keysGeneration.configureStep.allElections")}
                            />
                        </ReferenceInput>

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
                    title={
                        isAutomaticCeremony
                            ? t(
                                  "keysGeneration.configureStep.confirmdDialog.automaticCeremonyTitle"
                              )
                            : t("keysGeneration.configureStep.confirmdDialog.title")
                    }
                    handleClose={(result: boolean) => {
                        if (result) {
                            confirmCreateKeysCeremony()
                        }
                        setOpenConfirmationModal(false)
                    }}
                >
                    {isAutomaticCeremony
                        ? t(
                              "keysGeneration.configureStep.confirmdDialog.automaticCeremonyDescription"
                          )
                        : t("keysGeneration.configureStep.confirmdDialog.description")}
                </Dialog>
            </WizardStyles.ContentBox>
        </>
    )
}
