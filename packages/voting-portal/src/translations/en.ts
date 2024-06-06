// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        common: {
            goBack: "Go back",
        },
        breadcrumbSteps: {
            electionList: "Ballot List",
            ballot: "Ballot",
            review: "Review",
            confirmation: "Confirmation",
            audit: "Audit",
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            clearButton: "Clear selection",
            ballotHelpDialog: {
                title: "Information: Ballot screen",
                content:
                    "This screen shows the contest you are elegible to vote. You can make your section by activate the checkbox on the Candidate/Answer right. To reset your selections, click “<b>Clear selection</b>” button, to move to next step, click “<b>Next</b>” button bellow.",
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
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button bellow, and to audit your ballot click the “<b>Audit Ballot</b>” button bellow. Please note than once you submit your ballot, you have voted and you will not be issued another ballot for this Ballot.",
            descriptionNoAudit:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button bellow. Please note than once you submit your ballot, you have voted and you will not be issued another ballot for this Ballot.",
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
                    "<p>Auditing the ballot will spoil it and you will need to start the process of voting again if you want to cast your vote. The ballot audit process allows you to verify it's correctly encoded. Doing this process requires you to have important technical knowledge, so we do not recommend it if you do not know what you are doing.</p><p><b>If you just want to cast your ballot, click <u>Cancel</u> to go back to the review ballot screen.</b></p>",
                ok: "Yes, I want to DISCARD my ballot to audit it",
                cancel: "Cancel",
            },
        },
        confirmationScreen: {
            title: "Your vote has been cast",
            description:
                "The confirmation code bellow verifies that <b>your ballot has been cast successfully</b>. You can use this code to verify that your ballot has been counted.",
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
            ballotIdHelpDialog: {
                title: "Information: Ballot ID",
                content:
                    "The Ballot ID is a code that allows you to find your ballot in the ballot box, this ID is unique and doesn't contain information about your selections.",
                ok: "OK",
            },
            errorDialogPrintVoteReceipt: {
                title: "Error",
                content: "An error has occured, please try again",
                ok: "OK",
            },
        },
        auditScreen: {
            printButton: "Print",
            restartButton: "Start Voting",
            title: "Audit your Ballot",
            description: "To verify your ballot you will need. to follow the bellow steps:",
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
                '<a class="link" href="{{linkToBallotVerifier}}" target="_blank">Access to the ballot verifier</a>, a new tab will open in your browser.',
            step2HelpDialog: {
                title: "Audit ballot tutorial",
                content:
                    "To audit your ballot you will need to follow the steps shown in the tutorial, this includes the download of a desktop application used to verify the encrypted ballot independently from the website.",
                ok: "OK",
            },
            bottomWarning:
                "For security reason, when you audit your ballot, it need to be spoiled. To continue with the voting process, you need to click ‘<b>Start Voting</b>’ bellow.",
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
                    "Number of selected choices {{numSelected}} is more than the maximum {{max}}",
                selectedMin:
                    "Number of selected choices {{numSelected}} is less than the minimum {{min}}",
                maxSelectionsPerType:
                    "Number of selected choices {{numSelected}} for list {{type}} is more than the maximum {{max}}",
                underVote:
                    "Number of selected choices {{numSelected}} is less than the maximun {{max}}",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but question doesn't allow it",
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
            titleResult: "Result of your Ballot lookup",
            description: "Verify that your Ballot has been correctly emitted",
            locate: "Locate your Ballot",
            locateAgain: "Locate another Ballot",
            found: "Your ballot ID {{ballotId}} has been located",
            notFound: "Your ballot ID {{ballotId}} has not been located",
            contentDesc: "This is your Ballot content: ",
            wrongFormatBallotId: "Wrong format for Ballot ID",
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
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
