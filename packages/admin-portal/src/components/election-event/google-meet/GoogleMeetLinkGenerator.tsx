// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useState } from "react"
import {
    Dialog,
    DialogTitle,
    DialogContent,
    DialogActions,
    Button,
    TextField,
    Box,
    Typography,
    Alert,
    CircularProgress,
    IconButton,
    Snackbar,
} from "@mui/material"
import { ContentCopy, VideoCall } from "@mui/icons-material"
import { useTranslation } from "react-i18next"
import { useMutation } from "@apollo/client"
import { GENERATE_GOOGLE_MEET } from "../../../queries/GenerateGoogleMeet"

interface GoogleMeetLinkGeneratorProps {
    open: boolean
    onClose: () => void
    electionEventName?: string
}

export const GoogleMeetLinkGenerator: React.FC<GoogleMeetLinkGeneratorProps> = ({
    open,
    onClose,
    electionEventName = "",
}) => {
    const { t } = useTranslation()
    const [meetingTitle, setMeetingTitle] = useState(
        electionEventName ? `${electionEventName} - Meeting` : "Election Event Meeting"
    )
    const [meetingDescription, setMeetingDescription] = useState("")
    const [startDate, setStartDate] = useState("")
    const [startTime, setStartTime] = useState("")
    const [duration, setDuration] = useState("60") // minutes
    const [attendeeEmail, setAttendeeEmail] = useState("participant@example.com") // Mock email
    const [generatedLink, setGeneratedLink] = useState("")
    const [error, setError] = useState("")
    const [copySuccess, setCopySuccess] = useState(false)

    const [generateGoogleMeet, { loading: isGenerating }] = useMutation(GENERATE_GOOGLE_MEET, {
        onCompleted: (data) => {
            if (data.generate_google_meet.meet_link) {
                setGeneratedLink(data.generate_google_meet.meet_link)
                setError("")
            } else if (data.generate_google_meet.error_msg) {
                setError(data.generate_google_meet.error_msg)
            }
        },
        onError: (error) => {
            console.error("Error generating Google Meet link:", error)
            setError(error.message || "Failed to generate Google Meet link")
        },
    })

    const handleClose = () => {
        setGeneratedLink("")
        setError("")
        onClose()
    }

    const handleGenerateMeetLink = async () => {
        setError("")

        try {
            const startDateTime = new Date(`${startDate}T${startTime}`)
            const endDateTime = new Date(startDateTime.getTime() + parseInt(duration) * 60000)

            await generateGoogleMeet({
                variables: {
                    summary: meetingTitle,
                    description: meetingDescription,
                    startDateTime: startDateTime.toISOString(),
                    endDateTime: endDateTime.toISOString(),
                    timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
                    attendeeEmail: attendeeEmail,
                },
            })
        } catch (err: any) {
            console.error("Error generating Google Meet link:", err)
            setError(err.message || "Failed to generate Google Meet link")
        }
    }

    const copyToClipboard = async () => {
        try {
            await navigator.clipboard.writeText(generatedLink)
            setCopySuccess(true)
        } catch (err) {
            console.error("Failed to copy to clipboard:", err)
        }
    }

    const handleCopySuccessClose = () => {
        setCopySuccess(false)
    }

    // Set default date and time to current date/time + 1 hour
    React.useEffect(() => {
        if (open && !startDate) {
            const now = new Date()
            now.setHours(now.getHours() + 1)
            const date = now.toISOString().split("T")[0]
            const time = now.toTimeString().slice(0, 5)
            setStartDate(date)
            setStartTime(time)
        }
    }, [open, startDate])

    return (
        <>
            <Dialog open={open} onClose={handleClose} maxWidth="md" fullWidth>
                <DialogTitle>
                    <Box display="flex" alignItems="center" gap={1}>
                        <VideoCall color="primary" />
                        {t("googleMeet.title", "Generate Google Meet Link")}
                    </Box>
                </DialogTitle>
                <DialogContent>
                    <Box display="flex" flexDirection="column" gap={2} mt={1}>
                        {error && (
                            <Alert severity="error" onClose={() => setError("")}>
                                {error}
                            </Alert>
                        )}

                        {!generatedLink ? (
                            <>
                                <TextField
                                    label={t("googleMeet.meetingTitle", "Meeting Title")}
                                    value={meetingTitle}
                                    onChange={(e) => setMeetingTitle(e.target.value)}
                                    fullWidth
                                    required
                                />

                                <TextField
                                    label={t("googleMeet.description", "Description (Optional)")}
                                    value={meetingDescription}
                                    onChange={(e) => setMeetingDescription(e.target.value)}
                                    fullWidth
                                    multiline
                                    rows={2}
                                />

                                <Box display="flex" gap={2}>
                                    <TextField
                                        label={t("googleMeet.startDate", "Start Date")}
                                        type="date"
                                        value={startDate}
                                        onChange={(e) => setStartDate(e.target.value)}
                                        required
                                        InputLabelProps={{ shrink: true }}
                                        sx={{ flex: 1 }}
                                    />
                                    <TextField
                                        label={t("googleMeet.startTime", "Start Time")}
                                        type="time"
                                        value={startTime}
                                        onChange={(e) => setStartTime(e.target.value)}
                                        required
                                        InputLabelProps={{ shrink: true }}
                                        sx={{ flex: 1 }}
                                    />
                                </Box>

                                <TextField
                                    label={t("googleMeet.duration", "Duration (minutes)")}
                                    type="number"
                                    value={duration}
                                    onChange={(e) => setDuration(e.target.value)}
                                    required
                                    inputProps={{ min: 15, max: 480 }}
                                />

                                <TextField
                                    label={t("googleMeet.attendeeEmail", "Attendee Email")}
                                    value={attendeeEmail}
                                    onChange={(e) => setAttendeeEmail(e.target.value)}
                                    fullWidth
                                    type="email"
                                    helperText={t("googleMeet.attendeeEmailHelp", "Email for meeting participants (can be a mock email for testing)")}
                                />

                                <Typography variant="body2" color="text.secondary">
                                    {t(
                                        "googleMeet.note",
                                        "Note: This will create a calendar event in your Google Calendar with a Google Meet link. You'll need to sign in to your Google account."
                                    )}
                                </Typography>
                            </>
                        ) : (
                            <Box>
                                <Typography variant="h6" gutterBottom color="success.main">
                                    {t("googleMeet.success", "Google Meet Link Generated Successfully!")}
                                </Typography>
                                <Box
                                    display="flex"
                                    alignItems="center"
                                    gap={1}
                                    p={2}
                                    bgcolor="grey.100"
                                    borderRadius={1}
                                >
                                    <TextField
                                        value={generatedLink}
                                        fullWidth
                                        variant="outlined"
                                        size="small"
                                        InputProps={{
                                            readOnly: true,
                                        }}
                                    />
                                    <IconButton
                                        onClick={copyToClipboard}
                                        color="primary"
                                        title={t("googleMeet.copy", "Copy to clipboard")}
                                    >
                                        <ContentCopy />
                                    </IconButton>
                                </Box>
                                <Typography variant="body2" color="text.secondary" mt={1}>
                                    {t(
                                        "googleMeet.instructions",
                                        "Share this link with participants to join the meeting. The calendar event has been added to your Google Calendar."
                                    )}
                                </Typography>
                            </Box>
                        )}
                    </Box>
                </DialogContent>
                <DialogActions>
                    <Button onClick={handleClose}>{t("common.label.cancel", "Cancel")}</Button>
                    {!generatedLink && (
                        <Button
                            onClick={handleGenerateMeetLink}
                            variant="contained"
                            disabled={
                                isGenerating || !meetingTitle || !startDate || !startTime || !duration || !attendeeEmail
                            }
                            startIcon={
                                isGenerating ? <CircularProgress size={16} /> : <VideoCall />
                            }
                        >
                            {isGenerating
                                ? t("googleMeet.generating", "Generating...")
                                : t("googleMeet.generate", "Generate Meet Link")}
                        </Button>
                    )}
                </DialogActions>
            </Dialog>

            <Snackbar
                open={copySuccess}
                autoHideDuration={3000}
                onClose={handleCopySuccessClose}
                message={t("googleMeet.copied", "Link copied to clipboard!")}
            />
        </>
    )
}
