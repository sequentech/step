// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//

import BallotLocator from "../routes/BallotLocator"

// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        common: {
            goBack: "Go back",
            showMore: "Show More",
            showLess: "Show Less",
        },
        breadcrumbSteps: {
            electionList: "Ballot List",
            ballot: "Ballot",
            review: "Review",
            confirmation: "Confirmation",
            audit: "Audit",
        },
        footer: {
            poweredBy: "Powered by <sequent />",
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            clearButton: "Clear selection",
            ballotHelpDialog: {
                title: "Information: Ballot screen",
                content:
                    "This screen shows the contest you are elegible to vote. You can make your section by activate the checkbox on the Candidate/Answer right. To reset your selections, click “<b>Clear selection</b>” button, to move to next step, click “<b>Next</b>” button below.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Invalid or blank vote",
                content:
                    "Some of your answers will render the ballot in one or more questions invalid or blank.",
                ok: "Back and review",
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
            instructionsTitle: "Instructions",
            instructionsDescription: "Please follow these steps to cast your ballot:",
            step1Title: "1. Select your options",
            step1Description:
                "Choose your preferred candidates and answer the Ballot questions one by one as they appear. You can edit your ballot until you are ready to proceed.",
            step2Title: "2. Review your ballot",
            step2Description:
                "Once you are satisfied with your selections, we will encrypt your ballot and show you a final review of your choices. You will also receive a unique tracker ID for your ballot.",
            step3Title: "3. Cast your ballot",
            step3Description:
                "Cast your ballot: Finally, you can cast your ballot so it is properly registered. Alternatively, you can opt to audit and confirm that your ballot was correctly captured and encrypted.",
        },
        reviewScreen: {
            title: "Review your ballot",
            description:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below, and to audit your ballot click the “<b>Audit Ballot</b>” button below.",
            descriptionNoAudit:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below.",
            backButton: "Edit ballot",
            castBallotButton: "Cast your ballot",
            auditButton: "Audit ballot",
            reviewScreenHelpDialog: {
                title: "Information: Review Screen",
                content:
                    "This screen allows you to review your selections before casting your ballot.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Vote has not been cast",
                content:
                    "<p>This is your Ballot Tracker ID, but <b>your vote has not been cast yet</b>. If you try to track the ballot, you will not find it.</p><p>The reason we show the Ballot Tracker ID at this stage is to allow you to audit the correctness of the encrypted ballot before casting it.</p>",
                ok: "I accept my vote has NOT been cast",
                cancel: "Cancel",
            },
            auditBallotHelpDialog: {
                title: "Do you want to audit the ballot?",
                content:
                    "<p>Please note that auditing your ballot will void it, requiring you to restart the voting process. The audit process lets you verify that your ballot is correctly encoded, but it involves advanced technical steps. We recommend proceeding only if you are confident in your technical skills. If you just want to cast your ballot, click <u>Cancel</u> to go back to the review ballot screen.</b></p>",
                ok: "Yes, I want to DISCARD my ballot to audit it",
                cancel: "Cancel",
            },
            confirmCastVoteDialog: {
                title: "Are you sure you want to cast your vote?",
                content: "Your vote will no longer be editable once confirmed.",
                ok: "Yes, I want to CAST my vote",
                cancel: "Cancel",
            },
            error: {
                NETWORK_ERROR:
                    "There was a network problem. Please try again later or contact support for assistance.",
                UNABLE_TO_FETCH_DATA:
                    "There was a problem fetching the data. Please try again later or contact support for assistance.",
                LOAD_ELECTION_EVENT: "Cannot load election event. Please try again later.",
                CAST_VOTE:
                    "There was an unknown error while casting the vote. Please try again later or contact support for assistance.",
                CAST_VOTE_AreaNotFound:
                    "There was an error while casting the vote: Area not found. Please try again later or contact support for assistance.",
                CAST_VOTE_CheckStatusFailed:
                    "Election does not allow casting the vote. Election might be closed, archived or you might be trying to vote outside grace period.",
                CAST_VOTE_InternalServerError:
                    "An internal error occurred while casting the vote. Please try again later or contact support for assistance.",
                CAST_VOTE_QueueError:
                    "There was a problem processing your vote. Please try again later or contact support for assistance.",
                CAST_VOTE_Unauthorized:
                    "You are not authorized to cast a vote. Please contact support for assistance.",
                CAST_VOTE_ElectionEventNotFound:
                    "The election event could not be found. Please try again later or contact support for assistance.",
                CAST_VOTE_ElectoralLogNotFound:
                    "Your voting record could not be found. Please contact support for assistance.",
                CAST_VOTE_CheckPreviousVotesFailed:
                    "An error occurred while checking your voting status. Please try again later or contact support for assistance.",
                CAST_VOTE_GetClientCredentialsFailed:
                    "Failed to verify your credentials. Please try again later or contact support for assistance.",
                CAST_VOTE_GetAreaIdFailed:
                    "An error occurred verifying your voting area. Please try again later or contact support for assistance.",
                CAST_VOTE_GetTransactionFailed:
                    "An error occurred processing your vote. Please try again later or contact support for assistance.",
                CAST_VOTE_DeserializeBallotFailed:
                    "An error occurred reading your ballot. Please try again later or contact support for assistance.",
                CAST_VOTE_DeserializeContestsFailed:
                    "An error occurred reading your selections. Please try again later or contact support for assistance.",
                CAST_VOTE_PokValidationFailed:
                    "Failed to validate your vote. Please try again later or contact support for assistance.",
                CAST_VOTE_UuidParseFailed:
                    "An error occurred processing your request. Please try again later or contact support for assistance.",
                CAST_VOTE_unexpected:
                    "An unknown error occurred while casting the vote. Please try again later or contact support for assistance.",
                CAST_VOTE_timeout:
                    "Timeout error to cast the vote. Please try again later or contact support for assistance.",
                CAST_VOTE_InsertFailedExceedsAllowedRevotes:
                    "You have exceeded the revotes limit. Please try again later or contact support for assistance.",
                CAST_VOTE_CheckRevotesFailed:
                    "You have exceeded the allowed number of revotes. Please try again later or contact support for assistance.",
                CAST_VOTE_CheckVotesInOtherAreasFailed:
                    "You have voted in another area already. Please try again later or contact support for assistance.",
                CAST_VOTE_UnknownError:
                    "An unknown error occurred while casting the vote. Please try again later or contact support for assistance.",
                NO_BALLOT_SELECTION:
                    "The selection state for this election is not present. Please ensure you have selected your choices correctly or contact support.",
                NO_BALLOT_STYLE: "The ballot style is not available. Please contact support.",
                NO_AUDITABLE_BALLOT: "No auditable ballot is available. Please contact support.",
                INCONSISTENT_HASH:
                    "There was an error related to the ballot hashing process. BallotId: {{ballotId}} is not consistent with auditable Ballot Hash: {{auditableBallotHash}}. Please report this issue to support.",
                ELECTION_EVENT_NOT_OPEN: "The election event is closed. Please contact support.",
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
                UNKNOWN_ERROR:
                    "There was an error. Please try again later or contact support for assistance.",
                REAUTH_FAILED:
                    "Authentication failed. Please try again or contact support for assistance.",
                SESSION_EXPIRED: "Your session has expired. Please try again from the beginning.",
                CAST_VOTE_BallotIdMismatch: "The ballot id does not match with the cast vote.",
                SESSION_STORAGE_ERROR:
                    "Session storage is not available. Please try again or contact support.",
                PARSE_BALLOT_DATA_ERROR:
                    "There was an error parsing the ballot data. Please try again later or contact support for assistance.",
                NOT_VALID_BALLOT_DATA_ERROR:
                    "Ballot data is not valid. Please try again later or contact support for assistance.",
                FETCH_DATA_TIMEOUT_ERROR:
                    "Timeout error to fetch the data. Please try again later or contact support for assistance.",
                TO_HASHABLE_BALLOT_ERROR:
                    "Error converting to hashable ballot. Please try again later or contact support for assistance.",
                INTERNAL_ERROR:
                    "There was an internal error while casting the vote. Please try again later or contact support for assistance.",
            },
        },
        confirmationScreen: {
            title: "Your vote has been cast",
            description:
                "The confirmation code below verifies that <b>your ballot has been cast successfully</b>. You can use this code to verify that your ballot has been counted.",
            ballotId: "Ballot ID",
            printButton: "Print",
            finishButton: "Finish",
            verifyCastTitle: "Verify that your ballot has been cast",
            verifyCastDescription:
                "You can verify your ballot has been cast correctly at any moment using the following QR code:",
            confirmationHelpDialog: {
                title: "Information: Confirmation Screen",
                content:
                    "This screen shows that your vote was successfully cast. The information provided on this page allows you to verify that the ballot has been stored in ballot box , this process can be executed at any time during voting period and after the Ballot has been closed.",
                ok: "OK",
            },
            demoPrintDialog: {
                title: "Printing ballot",
                content: "Printing disabled in demo mode",
                ok: "OK",
            },
            demoBallotUrlDialog: {
                title: "Ballot Id",
                content: "Cannot use code, disabled in demo mode.",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Information: Ballot ID",
                content:
                    "The Ballot ID is a code that allows you to find your ballot in the ballot box, this ID is unique and doesn't contain information about your selections.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "Information: Ballot ID",
                content:
                    "<p>The Ballot ID is a code that allows you to find your ballot in the ballot box, this ID is unique and doesn't contain information about your selections.</p><p><b>Notice:</b> This voting booth is for demonstration purposes only. Your vote has NOT been cast.</p>",
                ok: "OK",
            },
            errorDialogPrintBallotReceipt: {
                title: "Error",
                content: "An error has occured, please try again",
                ok: "OK",
            },
            demoQRText: "Ballot tracker is disabled in demo mode",
        },
        auditScreen: {
            printButton: "Print",
            restartButton: "Start Voting",
            title: "Audit your Ballot",
            description: "To verify your ballot, please follow the steps below:",
            step1Title: "1. Download or copy the following information",
            step1Description:
                "Your <b>Ballot ID</b> that appears at the top of the screen and your encrypted ballot below:",
            step1HelpDialog: {
                title: "Copy the Encrypted Ballot",
                content:
                    "You can download or copy your encrypted ballot to audit the ballot and verify the encrypted content contains your selections.",
                ok: "OK",
            },
            downloadButton: "Download",
            step2Title: "2. Verify your ballot",
            step2Description:
                "<VerifierLink>Access to the ballot verifier</VerifierLink>, a new tab will open in your browser.",
            step2HelpDialog: {
                title: "Audit ballot tutorial",
                content:
                    "To audit your ballot you will need to follow the steps shown in the tutorial, this includes the download of a desktop application used to verify the encrypted ballot independently from the website.",
                ok: "OK",
            },
            bottomWarning:
                "For security reasons, when you audit your ballot, it needs to be spoiled. To continue with the voting process, you need to click ‘<b>Start Voting</b>’ below.",
        },
        electionSelectionScreen: {
            title: "Ballot list",
            description: "Select the Ballot you want to vote",
            chooserHelpDialog: {
                title: "Information: Ballot List",
                content:
                    "Welcome to the Voting Booth, this screen shows the list of Ballots you can cast a ballot. Ballots displayed in this list can be open to voting, scheduled, or closed. You will be able to access the ballot only if the voting period is open.",
                ok: "OK",
            },
            noResults: "No ballots for now.",
            demoDialog: {
                title: "Demo voting booth",
                content:
                    "You are entering a demo voting booth. <strong>Your vote will NOT be cast.</strong> This voting booth is for demonstration purposes only.",
                ok: "I accept my vote will Not be cast",
            },
            errors: {
                noVotingArea:
                    "Election area not assigned to voter. Please try again later or contact support for assistance.",
                networkError:
                    "There was a network problem. Please try again later or contact support for assistance.",
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
                    "Write-in exceed by {{numCharsExceeded}} the maximum number of chars. Requires fixing.",
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
                alert: "Selection marked will be considered invalid vote.",
            },
            page: {
                oopsWithStatus: "Oops! {{status}}",
                oopsWithoutStatus: "Oops! Unexpected Error",
                somethingWrong: "Something went wrong.",
            },
        },
        materials: {
            common: {
                label: "Support Materials",
                back: "Back to Ballot List",
                close: "Close",
                preview: "Preview",
            },
        },
        ballotLocator: {
            title: "Locate your Ballot",
            titleResult: "Result of your Ballot Lookup",
            description: "Verify that your Ballot has been correctly submitted",
            locate: "Locate your Ballot",
            locateAgain: "Locate another Ballot",
            found: "Your ballot ID {{ballotId}} has been located",
            notFound: "Your ballot ID {{ballotId}} has not been located",
            contentDesc: "This is your Ballot content: ",
            wrongFormatBallotId: "Wrong format for Ballot ID",
            ballotIdNotFoundAtFilter:
                "Not found, check that your Ballot ID is correct and belongs to this user.",
            filterByBallotId: "Filter by Ballot ID",
            totalBallots: "Total Ballots: {{total}}",
            steps: {
                lookup: "Locate your Ballot",
                result: "Result",
            },
            titleHelpDialog: {
                title: "Information: Ballot Locator screen",
                content:
                    "This screen allows the voter to find their vote by using the Ballot ID to retrieve it. This procedure enables checking that their ballot was correctly cast and that the recorded ballot coincides with the encrypted ballot they sent.",
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
