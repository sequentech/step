// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export interface MeetingData {
    summary: string
    description: string
    startDateTime: string
    endDateTime: string
    timeZone: string
    attendeeEmail: string
}

// Extend the Window interface to include gapi
declare global {
    interface Window {
        gapi: any
    }
}

/**
 * Generates a Google Meet link by creating a calendar event with Google Meet conference data
 * @param meetingData - The meeting information
 * @returns Promise<string> - The Google Meet link
 */
export const generateGoogleMeetLink = async (meetingData: MeetingData): Promise<string> => {
    try {
        // Initialize Google API (loads script if needed)
        await initializeGoogleApi()

        // Initialize the Google API client
        await window.gapi.client.init({
            apiKey: process.env.REACT_APP_GOOGLE_API_KEY || "your-api-key-here", // TODO: Retrieve from env or annotations in the back end.
            clientId: process.env.REACT_APP_GOOGLE_CLIENT_ID || "your-client-id-here",
            discoveryDocs: ["https://www.googleapis.com/discovery/v1/apis/calendar/v3/rest"],
            scope: "https://www.googleapis.com/auth/calendar.events",
        })

        // Check if user is signed in, if not, prompt for sign-in
        const authInstance = window.gapi.auth2.getAuthInstance()
        if (!authInstance.isSignedIn.get()) {
            await authInstance.signIn()
        }

        // Create the calendar event with Google Meet
        const event = {
            summary: meetingData.summary,
            description: meetingData.description,
            start: {
                dateTime: meetingData.startDateTime,
                timeZone: meetingData.timeZone,
            },
            end: {
                dateTime: meetingData.endDateTime,
                timeZone: meetingData.timeZone,
            },
            attendees: [
                {
                    email: meetingData.attendeeEmail,
                },
            ],
            conferenceData: {
                createRequest: {
                    requestId: `meet-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
                    conferenceSolutionKey: {
                        type: "hangoutsMeet",
                    },
                },
            },
        }

        // Insert the event into the calendar
        const response = await window.gapi.client.calendar.events.insert({
            calendarId: "primary",
            resource: event,
            conferenceDataVersion: 1,
        })

        // Extract the Google Meet link from the response
        const conferenceData = response.result.conferenceData
        if (!conferenceData || !conferenceData.entryPoints) {
            throw new Error("Failed to create Google Meet conference data")
        }

        const meetLink = conferenceData.entryPoints.find(
            (entry: any) => entry.entryPointType === "video"
        )?.uri

        if (!meetLink) {
            throw new Error("Google Meet link not found in the conference data")
        }

        return meetLink
    } catch (error: any) {
        console.error("Error in generateGoogleMeetLink:", error)
        
        // Provide more specific error messages
        if (error.error === "popup_blocked_by_browser") {
            throw new Error("Please allow popups for this site to sign in to Google")
        } else if (error.error === "access_denied") {
            throw new Error("Google Calendar access was denied. Please grant permission to create events.")
        } else if (error.status === 401) {
            throw new Error("Authentication failed. Please sign in to your Google account.")
        } else if (error.status === 403) {
            throw new Error("Permission denied. Please ensure you have Google Calendar access.")
        } else if (error.message) {
            throw new Error(error.message)
        } else {
            throw new Error("Failed to generate Google Meet link. Please try again.")
        }
    }
}

/**
 * Checks if the Google API is available and properly configured
 * @returns boolean - True if Google API is available
 */
export const isGoogleApiAvailable = (): boolean => {
    return typeof window !== "undefined" && !!window.gapi
}

/**
 * Loads Google API script dynamically if not already loaded
 */
const loadGoogleApiScript = (): Promise<void> => {
    return new Promise((resolve, reject) => {
        if (window.gapi) {
            resolve()
            return
        }

        const script = document.createElement('script')
        script.src = 'https://apis.google.com/js/api.js'
        script.onload = () => resolve()
        script.onerror = () => reject(new Error('Failed to load Google API script'))
        document.head.appendChild(script)
    })
}

/**
 * Initializes Google API if not already initialized
 * This can be called on app startup to preload the API
 */
export const initializeGoogleApi = async (): Promise<void> => {
    // Load the script first if not available
    await loadGoogleApiScript()

    if (!isGoogleApiAvailable()) {
        throw new Error("Google API not available")
    }

    return new Promise<void>((resolve, reject) => {
        window.gapi.load("client:auth2", {
            callback: resolve,
            onerror: reject,
        })
    })
}
