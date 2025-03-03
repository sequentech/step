// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, useEffect, useContext} from "react"
import Keycloak from "keycloak-js"
import {
    Box,
    Typography,
    TextField,
    Button,
    Card,
    CardContent,
    InputAdornment,
    IconButton,
} from "@mui/material"
import {styled} from "@mui/system"
import VisibilityIcon from "@mui/icons-material/Visibility"
import VisibilityOffIcon from "@mui/icons-material/VisibilityOff"
import {adminTheme, Header} from "@sequentech/ui-essentials"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useNavigate} from "react-router"
import {useNotify} from "react-admin"

// You would need to import your logo
// import Logo from "../assets/logo.png"

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

const LogoWrapper = styled(Box)(({theme}) => ({
    width: "100%",
    display: "flex",
    justifyContent: "center",
    marginBottom: theme.spacing(2),
}))

const LogoImage = styled("img")({
    width: 150,
    height: 150,
    objectFit: "contain",
})

const BackgroundWrapper = styled(Box)({
    width: "100%",
    height: "100vh",
    background:
        "linear-gradient(to bottom right, rgba(255, 255, 255, 0.8), rgba(200, 200, 200, 0.8))",
    backgroundSize: "cover",
    display: "flex",
    flexDirection: "column",
    justifyContent: "center",
    alignItems: "center",
    position: "relative",
    overflow: "hidden",
})

const LogoBackground = styled(Box)({
    position: "absolute",
    width: "100%",
    height: "100%",
    opacity: 0.1,
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    zIndex: 0,
})

const ContentWrapper = styled(Box)({
    zIndex: 1,
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    width: "100%",
    maxWidth: 600,
})

const VersionInfo = styled(Typography)({
    position: "absolute",
    top: 20,
    right: 20,
    fontSize: 14,
})

const Footer = styled(Typography)({
    position: "absolute",
    bottom: 20,
    fontSize: 12,
    opacity: 0.7,
})

export const SelectTenant = () => {
    const {isAuthenticated, initKeycloak} = useContext(AuthContext)
    const [tenant, setTenant] = useState("")
    const [isLoading, setIsLoading] = useState(false)
    const {globalSettings} = useContext(SettingsContext)
    const [error, setError] = useState<string | null>(null)
    const navigate = useNavigate()

    useEffect(() => {
        console.log("aa is auth", isAuthenticated)
    }, [isAuthenticated])

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
            console.error("Error checking realm existence:", error)
            return false
        }
    }

    const handleSubmit = async (e: React.SyntheticEvent) => {
        e.preventDefault()
        setIsLoading(true)
        setError(null)
        const tenantName = tenant.trim()

        try {
            const realmExists = await checkIfRealmExists(tenantName)
            if (!realmExists) {
                setError("Tenant does not exist")
                return
            }

            // Clear previous auth state
            localStorage.removeItem("token")
            sessionStorage.clear()

            // Init Keycloak with selected tenant
            await initKeycloak(tenantName)
        } catch (err) {
            setError("Authentication failed. Please try again.")
            console.error("Login error:", err)
        } finally {
            setIsLoading(false)
        }
    }

    return (
        <Box>
            <Header
                appVersion={{main: globalSettings.APP_VERSION}}
                appHash={{main: globalSettings.APP_HASH}}
                // userProfile={{
                //     firstName: authContext.firstName,
                //     username: authContext.username,
                //     email: authContext.email,
                //     openLink: authContext.openProfileLink,
                // }}
                // logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
                // languagesList={langList}
                // logoUrl={logoImg}
            />
            <BackgroundWrapper>
                <ContentWrapper>
                    <StyledCard>
                        <CardContent>
                            {/* <LogoWrapper>
                            <LogoImage src={Logo} alt="OVCS Logo" />
                        </LogoWrapper> */}

                            <Typography variant="h5" component="h1" align="center" gutterBottom>
                                OVCS ADMIN PORTAL
                            </Typography>

                            <Typography variant="h6" component="h2" align="center" gutterBottom>
                                Enter Tenant Name
                            </Typography>

                            <Box component="form" onSubmit={handleSubmit} sx={{mt: 2}}>
                                <TextField
                                    fullWidth
                                    label="Tenant Name"
                                    variant="outlined"
                                    margin="normal"
                                    value={tenant}
                                    onChange={(e) => setTenant(e.target.value)}
                                    required
                                    sx={{mb: 2}}
                                />

                                <StyledButton
                                    fullWidth
                                    variant="contained"
                                    type="submit"
                                    disabled={isLoading}
                                >
                                    {isLoading ? "Processing..." : "Continue"}
                                </StyledButton>
                            </Box>
                            {error ? <div>{error}</div> : null}
                        </CardContent>
                    </StyledCard>
                </ContentWrapper>

                <Footer>Powered by Sequent Tech Inc</Footer>
            </BackgroundWrapper>
        </Box>
    )
}

export default SelectTenant
