// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
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
import {ContentCopy, VideoCall} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {GENERATE_GOOGLE_MEET} from "../../../queries/GenerateGoogleMeet"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {GenerateGoogleMeetMutation} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {ZonedDateTimeField, zonedValueOf} from "@/components/timezones/ZonedDateTimeInput"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"
import type {ITimeZoneContext} from "@/components/timezones/useTimeZoneContext"

interface GoogleMeetLinkGeneratorProps {
    open: boolean
    onClose: () => void
    electionEventName?: string
    /** The event's zones: the meeting starts in its primary unless another is chosen. */
    timeZones?: Pick<ITimeZoneContext, "configured" | "primary">
}

export const GoogleMeetLinkGenerator: React.FC<GoogleMeetLinkGeneratorProps> = ({
    open,
    onClose,
    electionEventName = "",
    timeZones,
}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const [meetingTitle, setMeetingTitle] = useState(
        electionEventName ? `${electionEventName} - Meeting` : "Election Event Meeting"
    )
    const [meetingDescription, setMeetingDescription] = useState("")
    // The start as a wall time in a zone: the event's primary, else the viewer's.
    const [start, setStart] = useState({local: "", timezone: ""})
    const startZone = start.timezone || timeZones?.primary || service.myTimeZone
    const zonedStart = zonedValueOf(start.local, startZone, service)
    const [duration, setDuration] = useState("60") // minutes
    const [attendeeEmails, setAttendeeEmails] = useState("participant@example.com") // Mock emails
    const [generatedLink, setGeneratedLink] = useState("")
    const [error, setError] = useState("")
    const [copySuccess, setCopySuccess] = useState(false)
    const [isGenerating, setIsGenerating] = useState(false)

    const [generateGoogleMeet] = useMutation<GenerateGoogleMeetMutation>(GENERATE_GOOGLE_MEET, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.GOOGLE_MEET_LINK,
            },
        },
    })

    const handleClose = () => {
        setGeneratedLink("")
        setError("")
        onClose()
    }

    const handleGenerateMeetLink = async () => {
        setError("")
        setIsGenerating(true)
        try {
            if (!zonedStart) throw new Error("No start time")
            const startDateTime = new Date(zonedStart.scheduled_date)
            const endDateTime = new Date(startDateTime.getTime() + parseInt(duration) * 60000)
            let {data} = await generateGoogleMeet({
                variables: {
                    summary: meetingTitle,
                    description: meetingDescription,
                    startDateTime: startDateTime.toISOString(),
                    endDateTime: endDateTime.toISOString(),
                    timeZone: zonedStart.timezone,
                    attendeeEmails: attendeeEmails
                        .split(",")
                        .map((email) => email.trim())
                        .filter((email) => email.length > 0),
                },
            })

            if (data?.generate_google_meet?.meet_link) {
                setGeneratedLink(data.generate_google_meet.meet_link)
                setError("")
            } else {
                setError("Link is null.")
            }
        } catch (error: unknown) {
            console.error("Error generating Google Meet link:", error)
            // The handler's own message, rather than the status and the raw
            // response body this used to print at whoever was generating a link.
            const reason = getGraphQLActionErrorReason(error)
            setError(
                reason
                    ? `Failed to generate Google Meet link: ${reason}`
                    : "Failed to generate Google Meet link."
            )
        }
        setIsGenerating(false)
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

    // The default start: an hour from now, in the start's zone.
    React.useEffect(() => {
        if (open && !start.local) {
            setStart({
                local: service.instantToZoned(new Date(Date.now() + 3_600_000), startZone),
                timezone: startZone,
            })
        }
    }, [open, start.local, startZone, service])

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
                                    label={String(t("googleMeet.meetingTitle", "Meeting Title"))}
                                    value={meetingTitle}
                                    onChange={(e) => setMeetingTitle(e.target.value)}
                                    fullWidth
                                    required
                                />

                                <TextField
                                    label={String(
                                        t("googleMeet.description", "Description (Optional))")
                                    )}
                                    value={meetingDescription}
                                    onChange={(e) => setMeetingDescription(e.target.value)}
                                    fullWidth
                                    multiline
                                    rows={2}
                                />

                                <ZonedDateTimeField
                                    required
                                    label={t("lifecycle.input.meetingStart")}
                                    local={start.local}
                                    timezone={startZone}
                                    zones={timeZones?.configured}
                                    primary={timeZones?.primary}
                                    onChange={(_value, next) => setStart(next)}
                                />

                                <TextField
                                    label={String(t("googleMeet.duration", "Duration (minutes))"))}
                                    type="number"
                                    value={duration}
                                    onChange={(e) => setDuration(e.target.value)}
                                    required
                                    inputProps={{min: 15, max: 480}}
                                />

                                <TextField
                                    label={String(
                                        t("googleMeet.attendeeEmails", "Attendee Emails")
                                    )}
                                    value={attendeeEmails}
                                    onChange={(e) => setAttendeeEmails(e.target.value)}
                                    fullWidth
                                    helperText={t(
                                        "googleMeet.attendeeEmailHelp",
                                        "Comma-separated emails for meeting participants"
                                    )}
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
                                    {t(
                                        "googleMeet.success",
                                        "Google Meet Link Generated Successfully!"
                                    )}
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
                                        title={String(t("googleMeet.copy", "Copy to clipboard"))}
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
                                isGenerating ||
                                !meetingTitle ||
                                !zonedStart ||
                                !duration ||
                                !attendeeEmails
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
