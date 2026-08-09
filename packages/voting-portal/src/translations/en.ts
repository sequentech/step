// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        common: {
            goBack: "Back",
            showMore: "Show more",
            showLess: "Show less",
        },
        candidatesList: {
            collapseToggle: "Toggle list {{listTitle}}",
            showCandidates: "Show candidates",
            hideCandidates: "Hide candidates",
            selectedCandidate: "{{count}} candidate selected",
            selectedCandidates: "{{count}} candidates selected",
            expandAll: "Expand all",
            collapseAll: "Collapse all",
        },
        breadcrumbSteps: {
            electionList: "Ballots",
            ballot: "Ballot",
            review: "Review",
            confirmation: "Confirm",
            audit: "Audit",
        },
        footer: {
            poweredBy: "Powered by <1></1>",
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            clearButton: "Clear choices",
            ballotHelpDialog: {
                title: "About this screen",
                content:
                    "This screen shows the contest you are eligible to vote in. You can make your selection by activating the checkbox next to the candidate or answer. To reset your selections, click the “<b>Clear choices</b>” button. To move to the next step, click the “<b>Next</b>” button below.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Your vote is invalid or blank",
                content:
                    "Some of your answers will make one or more ballot questions invalid or blank",
                ok: "Review selection",
                continue: "Continue",
                cancel: "Cancel",
            },
            warningDialog: {
                title: "Review your ballot",
                content:
                    "Your ballot contains selections that may need your attention (such as selecting fewer options than allowed). Your ballot is valid and will be counted as submitted.",
                ok: "Back and review",
                continue: "Continue",
                cancel: "Cancel",
            },
        },
        startScreen: {
            startButton: "Start Voting",
            declineToVoteButton: "Decline to Vote",
            declineToVoteDialog: {
                title: "Confirm decline to vote",
                content:
                    "Are you sure you want to decline to vote?<br />You will go directly to review and your participation status will be saved as <b>Declined to vote</b>.",
                continue: "Decline to vote",
                cancel: "Cancel",
            },
            instructionsTitle: "How to vote",
            instructionsDescription: "Follow these steps to cast your ballot",
            step1Title: "1. Choose your options",
            step1Description:
                "Pick your preferred candidates and answer each ballot question as it appears. You can change your ballot anytime before casting your vote",
            step2Title: "2. Review your choices",
            step2Description:
                "When you’re happy with your selections, we’ll securely encrypt your ballot and show you a final review. You’ll also get a unique tracker ID for reference",
            step3Title: "3. Cast your ballot",
            step3Description:
                "When you’re ready, cast your ballot so it’s officially recorded. Or choose to audit first to confirm it was correctly captured and encrypted",
        },
        reviewScreen: {
            title: "Review your ballot",
            description:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below, and to audit your ballot click the “<b>Audit Ballot</b>” button below.",
            descriptionNoAudit:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below.",
            backButton: "Edit ballot",
            castBallotButton: "Cast ballot",
            auditButton: "Audit ballot",
            reviewScreenHelpDialog: {
                title: "About the review screen",
                content: "This screen lets you review your selections before casting your ballot",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Your vote has not been cast",
                content:
                    "<p>This is your Ballot Tracker ID, but <b>your vote has not been cast yet</b>. If you try to track it now, nothing will appear.</p><p>We show the Tracker ID at this stage so you can audit the encrypted ballot before casting it.</p>",
                ok: "I understand my vote is not cast",
                cancel: "Cancel",
            },
            auditBallotHelpDialog: {
                title: "Would you like to audit your ballot",
                content:
                    "<p>Auditing your ballot will void it and restart the voting process. Continue only if you’re comfortable with the advanced steps. Otherwise, click <u>Cancel</u> to go back.</p>",
                ok: "Yes, discard my ballot to audit",
                cancel: "Cancel",
            },
            confirmCastVoteDialog: {
                title: "Are you sure you want to cast your vote?",
                content: "After you confirm, your vote will be cast.",
                ok: "Yes, I want to cast my vote",
                cancel: "Cancel",
            },
            error: {
                NETWORK_ERROR:
                    "A network problem occurred. Please try again later or contact support",
                UNABLE_TO_FETCH_DATA:
                    "There was a problem fetching the data. Please try again later or contact support for assistance.",
                LOAD_ELECTION_EVENT: "Unable to load the election event. Please try again later",
                CAST_VOTE:
                    "An unknown error occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_AreaNotFound:
                    "There was an error while casting the vote: Area not found. Please try again later or contact support for assistance.",
                CAST_VOTE_CheckStatusFailed:
                    "This election does not allow casting a vote. It may be closed, archived, or outside the allowed voting period",
                CAST_VOTE_InternalServerError:
                    "An internal error occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_QueueError:
                    "There was a problem processing your vote. Please try again later or contact support",
                CAST_VOTE_Unauthorized:
                    "You are not authorized to cast a vote. Please contact support.",
                CAST_VOTE_ElectionEventNotFound:
                    "The election event could not be found. Please try again later or contact support.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Your voting record could not be found. Please contact support",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "An error occurred while checking your voting status. Please try again later or contact support",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Failed to verify your credentials. Please try again later or contact support",
                CAST_VOTE_GetAreaIdFailed:
                    "An error occurred while verifying your voting area. Please try again later or contact support",
                CAST_VOTE_GetTransactionFailed:
                    "An error occurred while processing your vote. Please try again later or contact support",
                CAST_VOTE_DeserializeBallotFailed:
                    "An error occurred while loading your ballot. Please try again later or contact support",
                CAST_VOTE_DeserializeContestsFailed:
                    "An error occurred while loading your selections. Please try again later or contact support.",
                CAST_VOTE_PokValidationFailed:
                    "Failed to validate your vote. Please try again later or contact support",
                CAST_VOTE_UuidParseFailed:
                    "An error occurred while processing your request. Please try again later or contact support.",
                CAST_VOTE_unexpected:
                    "An unknown error occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_timeout:
                    "A timeout occurred while casting your vote. Please try again later or contact support",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "You have exceeded the revote limit. Please contact support.",
                CAST_VOTE_CheckRevotesFailed:
                    "You have exceeded the allowed number of revotes. Please contact support",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "You have already voted in another area. Please contact support",
                CAST_VOTE_UnknownError:
                    "An unknown error occurred while casting your vote. Please try again later or contact support",
                NO_BALLOT_SELECTION:
                    "The selection state for this election is missing. Please check your choices or contact support",
                NO_BALLOT_STYLE: "This ballot is not available. Please contact support",
                NO_AUDITABLE_BALLOT:
                    "There is no ballot available for audit. Please contact support",
                INCONSISTENT_HASH:
                    "There was an error related to the ballot hashing process. BallotId: {{ballotId}} is not consistent with auditable Ballot Hash: {{auditableBallotHash}}. Please report this issue to support.",
                ELECTION_EVENT_NOT_OPEN: "This election is closed. You can no longer vote",
                PARSE_ERROR:
                    "There was an error parsing the ballot. Please try again later or contact support for assistance.",
                DESERIALIZE_AUDITABLE_ERROR:
                    "There was an error deserializing the auditable ballot. Please try again later or contact support for assistance.",
                DESERIALIZE_HASHABLE_ERROR:
                    "There was an error deserializing the hashable ballot. Please try again later or contact support for assistance.",
                CONVERT_ERROR:
                    "There was an error converting the ballot. Please try again later or contact support for assistance.",
                SERIALIZE_ERROR:
                    "There was an error serializing the ballot. Please try again later or contact support for assistance.",
                UNKNOWN_ERROR: "An error occurred. Please try again later or contact support",
                REAUTH_FAILED:
                    "Login failed. Your username or password may be incorrect. Please try again or contact support",
                SESSION_EXPIRED: "Your session has expired. Please start again",
                CAST_VOTE_BallotIdMismatch: "The ballot id does not match with the cast vote.",
                SESSION_STORAGE_ERROR:
                    "Session storage is not available. Please try again or contact support.",
                PARSE_BALLOT_DATA_ERROR:
                    "There was an error parsing the ballot data. Please try again later or contact support for assistance.",
                NOT_VALID_BALLOT_DATA_ERROR: "The ballot data is invalid. Please contact support",
                FETCH_DATA_TIMEOUT_ERROR:
                    "A timeout occurred while loading the data. Please try again",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error converting to hashable ballot. Please try again later or contact support for assistance.",
                INTERNAL_ERROR:
                    "There was an internal error while casting the vote. Please try again later or contact support for assistance.",
            },
            declineToVote: "Decline to vote",
        },
        confirmationScreen: {
            title: "Your vote has been cast",
            description:
                "Your ballot was cast successfully. Use the code below to verify that it was counted",
            ballotId: "Ballot ID",
            printButton: "Print",
            finishButton: "Finish",
            verifyCastTitle: "Verify that your ballot was cast",
            verifyCastDescription:
                "You can verify your ballot was cast correctly at any time using the QR code below",
            confirmationHelpDialog: {
                title: "About the confirmation screen",
                content:
                    "This screen confirms that your vote was successfully cast. The information here allows you to verify that your ballot was stored in the ballot box, both during the voting period and after it has closed",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Printing ballot",
                content: "Printing is disabled in demo mode",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "Ballot Id",
                content: "Code use is disabled in demo mode",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "About the Ballot ID",
                content:
                    "The Ballot ID is a unique code that lets you find your ballot in the ballot box. It does not contain any information about your choices.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "About the Ballot ID",
                content:
                    "The Ballot ID is a unique code that lets you find your ballot in the ballot box. It does not contain any information about your choices.",
                ok: "OK",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "An error occurred. Please try again",
                ok: "OK",
            },
            demoQRText: "Ballot tracker is disabled in demo mode",
        },
        auditScreen: {
            printButton: "Print",
            restartButton: "Start Voting",
            title: "Check your ballot",
            description: "To check your ballot, follow the steps below:",
            step1Title: "1. Save the following details:",
            step1Description:
                "your <b>Ballot ID</b> at the top of the screen and your encrypted ballot shown below",
            step1HelpDialog: {
                title: "Copy ballot code",
                content:
                    "You can download or copy your ballot code to verify that it correctly reflects your selections.",
                ok: "OK",
            },
            downloadButton: "Download",
            step2Title: "2. Check your ballot",
            step2Description:
                "Click <VerifierLink>Check your ballot code</VerifierLink>. It will open in a new tab",
            step2HelpDialog: {
                title: "How to check your ballot code",
                content:
                    "To check your ballot code, follow the steps in the how-to guide. This includes downloading a desktop application that lets you verify your ballot independently from the website.",
                ok: "OK",
            },
            bottomWarning:
                "For security reasons, when you audit your ballot, it needs to be spoiled. To continue with the voting process, you need to click ‘<b>Start Voting</b>’ below.",
        },
        electionSelectionScreen: {
            title: "Ballot list",
            description: "Select the ballot you want to vote on",
            chooserHelpDialog: {
                title: "About the Ballot list",
                content:
                    "This screen lists the ballots you can access. Some may be open, scheduled, or closed. You can only vote on ballots that are open",
                ok: "OK",
            },
            noResults: "No ballots for now.",
            resultsButton: "View results",
            demoDialog: {
                title: "Demo voting booth",
                content:
                    "You are entering a demo voting booth. <strong>Your vote will not be cast.</strong> This booth is for demonstration only.",
                ok: "I understand that my vote will not be cast",
            },
            errors: {
                noVotingArea:
                    "You are not listed as a voter in this election. Please contact support.",
                networkError:
                    "A network problem occurred. Please try again later or contact support",
                unableToFetchData:
                    "There was a problem fetching the data. Please try again later or contact support for assistance.",
                noElectionEvent:
                    "Election event doesn’t exist. Please try again later or contact support for assistance.",
                ballotStylesEmlError:
                    "There was an error with the publish ballot style. Please try again later or contact support for assistance.",
                obtainingElectionFromID:
                    "There was an error obtaining elections associated with the following election IDs: {{electionIds}}. Please try again later or contact support for assistance.",
            },
            alerts: {
                noElections:
                    "There are no elections you can vote for. This could be because the area doesn’t have any contest associated. Please try again later or contact support for assistance.",
                electionEventNotPublished:
                    "The election event hasn’t been published yet. Please try again later or contact support for assistance.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write-in doesn't end on 0",
                writeInCharsExceeded:
                    "The write-in exceeds the maximum length by {{numCharsExceeded}} characters. Please shorten it.",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            implicit: {
                selectedMax:
                    "Overvote: Number of selected choices {{numSelected}} is more than the maximum {{max}}",
                selectedMin:
                    "Number of selected choices {{numSelected}} is less than the minimum {{min}}",
                maxSelectionsPerType:
                    "Number of selected choices {{numSelected}} for list {{type}} is more than the maximum {{max}}",
                underVote:
                    "Undervote: Number of selected choices {{numSelected}} is less than the maximum {{max}}",
                overVoteDisabled:
                    "Maximum reached: You have selected the maximum {{numSelected}} choices. To change your selection, please deselect another option first.",
                blankVote: "Blank Vote: 0 choices selected",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but question doesn't allow it",
                alert: "This selection will be counted as an invalid vote",
            },
            page: {
                oopsWithStatus: "Oops! {{status}}",
                oopsWithoutStatus: "Oops! Unexpected Error",
                somethingWrong: "Something went wrong.",
                certAuthFailedTitle: "Certificate Authentication Failed",
                certAuthFailedMessage:
                    "Your certificate could not be verified. Please check that you are using a valid voter certificate and try again.",
            },
        },
        materials: {
            common: {
                label: "Support Materials",
                back: "Back to ballot list",
                close: "Close",
                preview: "Preview",
            },
        },
        ballotLocator: {
            title: "Find your Ballot",
            titleResult: "Your Ballot Lookup Results",
            description: "Confirm your ballot was cast correctly",
            locate: "Find your Ballot",
            locateAgain: "Find another Ballot",
            found: "Your ballot ID {{ballotId}} has been found",
            notFound: "Your ballot ID {{ballotId}} was not found",
            ambiguous:
                "More than one of your ballots matches {{ballotId}}. Use the full ballot ID.",
            contentDesc: "This is your Ballot content: ",
            wrongFormatBallotId: "Invalid Ballot ID format",
            ballotIdNotFoundAtFilter:
                "Not found, check that your Ballot ID is correct and belongs to this user.",
            filterByBallotId: "Filter by Ballot ID",
            totalBallots: "Total Ballots: {{total}}",
            steps: {
                lookup: "Find your Ballot",
                result: "Result",
            },
            titleHelpDialog: {
                title: "About the Ballot Finder Screen",
                content:
                    "The Ballot Finder screen lets you enter your Ballot ID to locate your vote and confirm it was recorded correctly.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Ballot Locator",
            },
            column: {
                statement_kind: "Statement kind",
                statement_timestamp: "Statement Timestamp",
                username: "Username",
                ballot_id: "Ballot ID",
                message: "Message",
            },
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
