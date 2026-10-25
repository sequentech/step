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
    ListKeysCeremonyQuery,
    TrusteeNamesQuery,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Trustee,
} from "@/gql/graphql"
import {
    useRefresh,
    SimpleForm,
    TextInput,
    CheckboxGroupInput,
    useGetOne,
    useNotify,
    useTranslate,
    ValidationErrorMessage,
    AutocompleteInput,
    ReferenceInput,
    BooleanInput,
} from "react-admin"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {useApolloClient, useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {CREATE_KEYS_CEREMONY} from "@/queries/CreateKeysCeremony"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Dialog} from "@sequentech/ui-essentials"
import {EElectionEventCeremoniesPolicy, IElectionEventPresentation} from "@sequentech/ui-core"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {IPermissions} from "@/types/keycloak"
import {Clear} from "@mui/icons-material"
import {CreateKeysError} from "@/types/ceremonies"
import {electionSearchFilter} from "@/services/ElectionSearch"

type TrusteeChoice = Pick<Sequent_Backend_Trustee, "id" | "name">

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
    setCurrentCeremony,
    electionEvent,
    openCeremonyStep,
    goBack,
}) => {
    const {t, i18n} = useTranslation()
    const client = useApolloClient()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const translate = useTranslate()
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
    // These choices only need public names/IDs. The generic resource is not
    // exposed by the board's default introspection role.
    const {
        data: trusteesData,
        loading: trusteesLoading,
        error: trusteesError,
    } = useQuery<TrusteeNamesQuery>(GET_TRUSTEES_NAMES, {
        variables: {tenantId: electionEvent.tenant_id},
        context: {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}},
    })
    const trusteeList = trusteesData?.sequent_backend_trustee
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
    const [filterTrustees, setFilterTrustees] = useState<string>("")
    const [filteredTrustees, setFilteredTrustees] = useState<TrusteeChoice[] | undefined>()

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
            trusteeList?.filter((trustee: TrusteeChoice) =>
                trustee?.name?.toLowerCase().includes(filterTrustees)
            ) ?? []
        )
    }, [filterTrustees, trusteeList])

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
            setErrors(t("keysGeneration.configureStep.errorCreatingCeremony", {error: "error"}))
            return null
        }
        return data?.create_keys_ceremony?.keys_ceremony_id ?? null
    }

    // Called by the confirmation dialog to create the Keys Ceremony
    const confirmCreateKeysCeremony = async () => {
        if (isLoading || newId || trusteesLoading || trusteesError || !trusteeList) {
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
            if (!keysCeremonyId) {
                notify(t("keysGeneration.configureStep.errorCreatingCeremony", {error: "error"}), {
                    type: "error",
                })
                return
            }
            // The mutation has committed. Keep its ID even if the follow-up read
            // fails, so submitting again cannot create a second ceremony.
            setNewId(keysCeremonyId)
            const {data} = await client.query<ListKeysCeremonyQuery>({
                query: LIST_KEYS_CEREMONY,
                variables: {electionEventId: electionEvent.id},
                context: {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}},
                fetchPolicy: "network-only",
            })
            const created =
                data.list_keys_ceremony?.items.filter(
                    (ceremony) =>
                        ceremony?.id === keysCeremonyId &&
                        ceremony.tenant_id === electionEvent.tenant_id &&
                        ceremony.election_event_id === electionEvent.id
                ) ?? []
            if (created.length !== 1) {
                throw new Error("The created ceremony could not be loaded for this election event.")
            }
            setCurrentCeremony(created[0] as Sequent_Backend_Keys_Ceremony)
            openCeremonyStep()
            notify(t("keysGeneration.configureStep.createCeremonySuccess"), {type: "success"})
            refresh()
        } catch (error) {
            setErrors(
                t("keysGeneration.configureStep.errorCreatingCeremony", {
                    error: error instanceof Error ? error.message : "error",
                })
            )
        } finally {
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
    const trusteeListValidator = (
        value: string[],
        values: FieldValues
    ): ValidationErrorMessage | null => {
        const length = value && value ? value.length : 0
        const requiredTrustees = Number(values.threshold)
        if (length < requiredTrustees) {
            return t("keysGeneration.configureStep.errorMinTrustees", {
                selected: length,
                threshold: requiredTrustees,
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

    const visibleError =
        errors ??
        (trusteesError
            ? t("keysGeneration.configureStep.errorCreatingCeremony", {
                  error: trusteesError.message,
              })
            : null)

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
                                disabled={
                                    isLoading ||
                                    !!newId ||
                                    trusteesLoading ||
                                    !!trusteesError ||
                                    !trusteeList
                                }
                                icon={<ArrowForwardIosIcon />}
                                label={String(t("keysGeneration.configureStep.create"))}
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
                            sx={{"& .MuiInputLabel-root.Mui-error": {color: "error.dark"}}}
                            label={String(t("keysGeneration.configureStep.threshold"))}
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
                                label={String(
                                    t("keysGeneration.configureStep.automaticCeremonyToggle")
                                )}
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
                                        label={String(
                                            t("keysGeneration.configureStep.filterTrustees")
                                        )}
                                        value={filterTrustees}
                                        type="text"
                                        onChange={(e) => setFilterTrustees(e.target.value)}
                                        endAdornment={
                                            <InputAdornment position="end">
                                                <IconButton
                                                    onClick={() => setFilterTrustees("")}
                                                    aria-label={translate(
                                                        "ra.action.clear_input_value"
                                                    )}
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
                            label={String(t("electionScreen.common.title"))}
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
                            sort={{field: "external_id", order: "ASC"}}
                        >
                            <AutocompleteInput
                                className="election-selector"
                                optionText={aliasRenderer}
                                filterToQuery={electionSearchFilter}
                                debounce={100}
                                emptyText={t("keysGeneration.configureStep.allElections")}
                            />
                        </ReferenceInput>

                        {visibleError ? (
                            <WizardStyles.ErrorMessage variant="body2" className="keys-error">
                                {visibleError}
                            </WizardStyles.ErrorMessage>
                        ) : null}
                        {isLoading || trusteesLoading ? <CircularProgress /> : null}
                    </WizardStyles.MainContent>
                </SimpleForm>
                <Dialog
                    variant="warning"
                    open={openConfirmationModal}
                    ok={String(t("keysGeneration.configureStep.confirmdDialog.ok"))}
                    cancel={String(t("keysGeneration.configureStep.confirmdDialog.cancel"))}
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
