// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, useContext} from "react"
import {
    Box,
    Typography,
    TextField,
    Button,
    Card,
    CardContent,
    Snackbar,
    Alert,
    CircularProgress,
    Stack,
} from "@mui/material"
import {styled} from "@mui/system"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useNavigate} from "react-router"
import {adminTheme, Header} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {ITenantTheme} from "@sequentech/ui-core"
import {AppBar} from "react-admin"

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    min-width: 100vw;
    ${({css}) => css}
`

export const StyledButton = styled(Button)`
    z-index: 1;
    position: relative;
    background: ${({theme}) => theme.palette.brandColor} !important;
    color: white !important;
    border-radius: 5px;
    border: none;
    display: flex;
    outline: "none";
    overflow: hidden;

    &:hover,
    &:focus,
    &:active {
        color: ${({theme}) => theme.palette.white} !important;
        background: ${({theme}) => theme.palette.brandColor} !important;
        box-shadow: none !important;
    }
`

const StyledCard = styled(Card)(({theme}) => ({
    width: "100%",
    padding: theme.spacing(2),
    boxShadow: "0px 4px 12px rgba(0, 0, 0, 0.1)",
    borderRadius: theme.spacing(1),
}))

const BackgroundWrapper = styled(Box)(({theme}) => ({
    width: "100%",
    height: "100vh",
    background: theme.palette.lightBackground,
    backgroundSize: "cover",
    display: "flex",
    flexDirection: "column",
    justifyContent: "center",
    alignItems: "center",
    position: "relative",
    overflow: "hidden",
}))

const ContentWrapper = styled(Box)({
    zIndex: 1,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    width: "100%",
    maxWidth: 600,
})

const Footer = styled(Typography)({
    position: "absolute",
    bottom: 20,
    fontSize: 12,
    opacity: 0.7,
})

/**
 * The `SelectTenant` component allows users to select a tenant by entering the tenant name.
 *
 * This component performs the following tasks:
 * - Redirects to the main app if the user is already authenticated.
 * - Provides a form for users to enter the tenant name.
 * - Validates the tenant name and checks if the tenant exists in the database.
 * - Checks if the Keycloak realm associated with the tenant exists.
 * - Stores the tenant ID in local storage.
 * - Initializes Keycloak without auto-redirect.
 * - Redirects to the login page if not already authenticated.
 * - Displays error messages if any step fails.
 *
 * @component
 * @returns {JSX.Element} The rendered component.
 *
 * @example
 * ```tsx
 * import { SelectTenant } from './SelectTenant';
 *
 * function App() {
 *   return <SelectTenant />;
 * }
 *
 * export default App;
 * ```
 */
export const SelectTenant = () => {
    const {t} = useTranslation()
    const {isAuthenticated, initKeycloak} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const [tenant, setTenant] = useState("")
    const [loading, setLoading] = useState(true)
    const [isLoading, setIsLoading] = useState(false)
    const [intents, setIntents] = useState(0)
    const [error, setError] = useState<string | null>(null)
    const [open, setOpen] = useState<boolean>(false)
    const [isDisabled, setIsDisabled] = useState(false)
    const navigate = useNavigate()
    const [logoImg, setLogoImg] = useState<string | undefined>(BlankLogoImg)
    const [css, setCss] = useState<string>("")

    /**
     * Fetches the default tenant's information from the backend and updates the logo and CSS state.
     *
     * This function sends a POST request to the HASURA_URL endpoint with a GraphQL query to fetch
     * the tenant details by ID. It then extracts the logo URL and CSS from the tenant's annotations
     * and updates the corresponding state variables.
     *
     * @async
     * @function getDefaultTenant
     *
     * @returns {Promise<void>} A promise that reso lves when the tenant information has been fetched and the state has been updated.
     *
     * @throws {Error} Throws an error if the fetch request fails or if the response contains errors.
     */
    const getDefaultTenant = async () => {
        const response = await fetch(globalSettings.HASURA_URL, {
            method: "POST",
            headers: {"Content-Type": "application/json"},
            body: JSON.stringify({
                query: `
                query GetTenantById($id: uuid!) {
                    sequent_backend_tenant(where: {id: {_eq: $id}}) {
                        id
                        slug
                        annotations
                        }
                        }
                        `,
                variables: {id: globalSettings.DEFAULT_TENANT_ID},
            }),
        })
        const {data, errors} = await response.json()
        const newLogoState = (
            data?.sequent_backend_tenant[0].annotations as ITenantTheme | undefined
        )?.logo_url
        const newCss = (data?.sequent_backend_tenant[0].annotations as ITenantTheme | undefined)
            ?.css
        setLogoImg(errors?.length > 0 ? SequentLogo : (newLogoState ?? SequentLogo))
        setCss(newCss ?? "")
    }

    useEffect(() => {
        // Check for previous attempts
        const nextAttempt = localStorage.getItem("next-tenant-attempt")
        if (nextAttempt) {
            const now = new Date()
            const attemptTime = new Date(nextAttempt)
            const diffMinutes = (now.getTime() - attemptTime.getTime()) / (1000 * 60)

            if (diffMinutes < 5) {
                setError("Too many attempts. Please try again later.")
                setOpen(true)
                setIsDisabled(true)
            } else {
                localStorage.removeItem("next-tenant-attempt")
                setIsDisabled(false)
            }
        }

        setTimeout(() => {
            setLoading(false)
        }, 500)
    }, [])

    useEffect(() => {
        if (isAuthenticated) {
            navigate("/") // Redirect to the app if already authenticated
        }
    }, [isAuthenticated, navigate])

    useEffect(() => {
        if (globalSettings) {
            getDefaultTenant() // Redirect to the app if already authenticated
        }
    }, [globalSettings])

    const handleClose = (event: React.SyntheticEvent | Event) => {
        setOpen(false)
    }

    /**
     * Check if a Keycloak realm exists
     * @param baseUrl - The Keycloak server URL
     * @param realmName - The name of the realm to check
     * @returns Promise resolving to true if realm exists, false otherwise
     */
    /**
     * Check if a realm exists using a simple fetch request instead of initializing Keycloak
     * This prevents unwanted redirects to Keycloak
     */
    const checkIfRealmExists = async (realmName: string): Promise<boolean> => {
        try {
            // Remove trailing slash if present
            const baseUrl = globalSettings.KEYCLOAK_URL.endsWith("/")
                ? globalSettings.KEYCLOAK_URL.slice(0, -1)
                : globalSettings.KEYCLOAK_URL

            // Use the well-known configuration endpoint which is publicly accessible
            // This endpoint exists for all valid realms and doesn't trigger redirects
            const configUrl = `${baseUrl}/realms/${realmName}/.well-known/openid-configuration`

            const response = await fetch(configUrl, {
                method: "GET",
                headers: {
                    Accept: "application/json",
                },
            })

            // If we get a successful response, the realm exists
            return response.ok
        } catch (error) {
            return false
        }
    }

    /**
     * Handles the form submission for selecting a tenant.
     *
     * @param {React.SyntheticEvent} e - The form submission event.
     *
     * This function performs the following steps:
     * 1. Prevents the default form submission behavior.
     * 2. Sets the loading state to true and clears any previous errors.
     * 3. Trims the tenant name and checks if it is not empty.
     * 4. Sends a GraphQL query to check if the tenant exists in the database.
     * 5. If the tenant exists, retrieves the tenant ID.
     * 6. Checks if the realm associated with the tenant exists.
     * 7. Stores the tenant ID in local storage.
     * 8. Initializes Keycloak without auto-redirect.
     * 9. If not already authenticated, manually redirects to the login page.
     *
     * If any error occurs during these steps, it sets the error message, stops the loading state, and opens an error dialog.
     *
     * @throws {Error} If the tenant name is empty, tenant is not found, tenant realm is not found, or any other error occurs during the process.
     */
    const handleSubmit = async (e: React.SyntheticEvent) => {
        e.preventDefault()
        setIsLoading(true)
        setError(null)

        if (intents >= 5) {
            setError("Too many attempts. Please try again later.")
            setIsLoading(false)
            setOpen(true)
            setIsDisabled(true)
            localStorage.setItem("next-tenant-attempt", new Date().toISOString())
            return
        } else {
            setIntents(intents + 1)
        }

        const slug = tenant.trim()
        if (!slug) {
            setError("Please enter a tenant name")
            setIsLoading(false)
            return
        }

        try {
            // First check if tenant exists in database
            const response = await fetch(globalSettings.HASURA_URL, {
                method: "POST",
                headers: {"Content-Type": "application/json"},
                body: JSON.stringify({
                    query: `
                    query GetTenantBySlug($slug: String!) {
                        sequent_backend_tenant(where: {slug: {_eq: $slug}}) {
                            id
                            slug
                        }
                    }
                `,
                    variables: {slug},
                }),
            })
            const {data, errors} = await response.json()

            if (errors) {
                throw new Error(errors[0].message)
            }

            if (!data?.sequent_backend_tenant?.length) throw new Error("Tenant not found")

            // if we are here, a tenant with the name slug has been found
            const tenantId = data.sequent_backend_tenant[0].id

            // Check if the realm exists
            const realmExists = await checkIfRealmExists(`tenant-${tenantId}`)

            if (!realmExists) throw new Error("Tenant realm not found")

            // Initialize Keycloak without auto-redirect
            const initSuccess = await initKeycloak(tenantId)
        } catch (err: any) {
            setError(err.message)
            setIsLoading(false)
            setOpen(true)
        }
    }

    return (
        <>
            {loading ? (
                <CircularProgress />
            ) : (
                <StyledApp css={css} className="select-tenant">
                    <Header
                        appVersion={{main: globalSettings.APP_VERSION}}
                        appHash={{main: globalSettings.APP_HASH}}
                        logoUrl={logoImg}
                    />
                    <BackgroundWrapper>
                        <ContentWrapper>
                            <StyledCard>
                                <CardContent>
                                    <Typography
                                        variant="h5"
                                        component="h1"
                                        align="center"
                                        gutterBottom
                                    >
                                        Sequent Admin Portal
                                    </Typography>

                                    <Typography
                                        variant="h6"
                                        component="h2"
                                        align="center"
                                        gutterBottom
                                    >
                                        {t("common.label.selectTenant")}
                                    </Typography>

                                    <Box component="form" onSubmit={handleSubmit} sx={{mt: 2}}>
                                        <TextField
                                            fullWidth
                                            label={String(t("common.label.tenantName"))}
                                            variant="outlined"
                                            margin="normal"
                                            value={tenant}
                                            onChange={(e) => setTenant(e.target.value)}
                                            required
                                            disabled={isDisabled}
                                            sx={{mb: 2}}
                                        />

                                        <StyledButton
                                            fullWidth
                                            variant="contained"
                                            type="submit"
                                            disabled={isLoading}
                                        >
                                            {isLoading
                                                ? t("common.label.processing")
                                                : t("common.label.continue")}
                                        </StyledButton>
                                    </Box>
                                </CardContent>
                            </StyledCard>
                        </ContentWrapper>

                        <Footer>Powered by Sequent Tech Inc</Footer>
                    </BackgroundWrapper>
                    <Snackbar
                        anchorOrigin={{vertical: "bottom", horizontal: "center"}}
                        open={open}
                        onClose={handleClose}
                        autoHideDuration={2000}
                    >
                        <Alert
                            severity="error"
                            variant="filled"
                            sx={{width: "100%", borderRadius: 2, paddingX: 8}}
                        >
                            <Box>{error}</Box>
                        </Alert>
                    </Snackbar>
                </StyledApp>
            )}
        </>
    )
}

export default SelectTenant
