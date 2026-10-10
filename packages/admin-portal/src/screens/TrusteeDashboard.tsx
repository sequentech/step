// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {Alert, Box, Button, Card, CardContent, TextField, Typography} from "@mui/material"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"

interface TrusteeBoard {
    name: string
    kind: "DKG" | "TALLY"
    parent: string | null
}

export const TrusteeDashboard = () => {
    const auth = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const platform = globalSettings.HARVEST_URL || "http://localhost:8400"
    const [boards, setBoards] = useState<TrusteeBoard[]>([])
    const [error, setError] = useState("")
    const [loading, setLoading] = useState(false)
    const [loaded, setLoaded] = useState(false)
    const refresh = async () => {
        setLoading(true)
        setError("")
        try {
            const token = localStorage.getItem("token")?.split(":")[0]
            if (!token) throw new Error("Sign in as a trustee to list assigned boards.")
            const response = await fetch(`${platform.replace(/\/$/, "")}/trustee/boards`, {
                headers: {Authorization: `Bearer ${token}`},
            })
            if (!response.ok)
                throw new Error(`Could not read assigned boards (${response.status}).`)
            const result = (await response.json()) as {boards: TrusteeBoard[]}
            if (!Array.isArray(result.boards)) throw new Error("Invalid board response.")
            setBoards(result.boards)
            setLoaded(true)
        } catch (error) {
            setError(error instanceof Error ? error.message : String(error))
        } finally {
            setLoading(false)
        }
    }
    return (
        <Box sx={{p: 3}}>
            <Typography variant="h4" component="h1">
                Wbraid trustee
            </Typography>
            <Typography sx={{my: 2}}>
                Run the native trustee tool with your local keys file. It discovers your assigned
                boards, verifies their messages and resumes from its local session stores.
            </Typography>
            <Card>
                <CardContent>
                    <Typography variant="h6" component="h2">
                        Local trustee tool
                    </Typography>
                    <Typography
                        component="pre"
                        sx={{whiteSpace: "pre-wrap"}}
                    >{`trustee generate --trustee-config trustee.toml
trustee start --trustee-config trustee.toml`}</Typography>
                    <Typography>
                        Register both public keys printed by generate. Keep a private backup of
                        trustee.toml and the message_store directory.
                    </Typography>
                    <Typography sx={{mt: 2}}>
                        For a manual ceremony, download its custody request and create a proof to
                        upload to the portal:
                    </Typography>
                    <Typography
                        component="pre"
                        sx={{whiteSpace: "pre-wrap"}}
                    >{`trustee attest --request custody-request.json --trustee-config trustee.toml --output custody-proof.json`}</Typography>
                    <Typography>
                        For a tally restore, add --tally-session-id with the tally session's ID. The
                        proof contains a signature and public metadata.
                    </Typography>
                </CardContent>
            </Card>
            <Typography variant="h6" component="h2" sx={{mt: 3}}>
                Boards assigned to {auth.trustee || "your trustee"}
            </Typography>
            <TextField
                label="Platform URL"
                value={platform}
                InputProps={{readOnly: true}}
                sx={{my: 2, mr: 2}}
            />
            <Button onClick={refresh} disabled={loading}>
                {loading ? "Loading boards…" : "Refresh boards"}
            </Button>
            {error && <Alert severity="error">{error}</Alert>}
            {loaded && boards.length === 0 && (
                <Typography>No boards are currently assigned.</Typography>
            )}
            {boards.map((board) => (
                <Card key={board.name} sx={{my: 1}}>
                    <CardContent>
                        <Typography>{board.name}</Typography>
                        <Typography>
                            {board.kind}
                            {board.parent ? ` · Parent: ${board.parent}` : ""}
                        </Typography>
                    </CardContent>
                </Card>
            ))}
        </Box>
    )
}
