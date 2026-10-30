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
        a11y: {
            skipToContent: "Skip to main content",
            helpAbout: "Help about {{topic}}",
            previewMaterial: "Preview {{title}}",
            ballotsTable: "Ballots",
            ballotLocatorTabs: "Ballot tracking sections",
            ballotIdLabel: "Tracking ID",
            votingProgress: "Voting progress",
            stepOf: "Step {{current}} of {{total}}",
            selectUpTo_one: "Select up to {{count}} option",
            selectUpTo_many: "Select up to {{count}} options",
            selectUpTo_other: "Select up to {{count}} options",
            selectExactly_one: "Select {{count}} option",
            selectExactly_many: "Select {{count}} options",
            selectExactly_other: "Select {{count}} options",
            selectBetween: "Select between {{min}} and {{max}} options",
        },
        candidatesList: {
            collapseToggle: "Toggle list {{listTitle}}",
            showCandidates: "Show candidates",
            hideCandidates: "Hide candidates",
            selectedCandidates_one: "{{count}} candidate selected",
            // `_many` is selected only in es/cat/fr, for exact multiples of a million. English never
            // selects it, but the other bundles are typed `TranslationType = typeof englishTranslation`,
            // so the key has to be declared here before they can carry it.
            selectedCandidates_many: "{{count}} candidates selected",
            selectedCandidates_other: "{{count}} candidates selected",
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
        contest: {
            acclamation: {
                description:
                    "This contest was decided by acclamation. Its candidates are elected without a vote, so no option can be selected and no vote is recorded for it.",
            },
        },
        votingScreen: {
            backButton: "Back",
            reviewButton: "Next",
            clearButton: "Clear selections",
            ballotHelpDialog: {
                title: "About this screen",
                content:
                    "This screen shows the contests you are eligible to vote in. You can make your selection by activating the checkbox to the right of the Candidate/Answer. To reset your selections, click the “<b>Clear selections</b>” button; to move to the next step, click the “<b>Next</b>” button below.",
                ok: "OK",
            },
            nonVotedDialog: {
                title: "Your vote is invalid or blank",
                content: "Some of your answers will make one or more contests invalid or blank",
                ok: "Review selections",
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
            blankBallotDialog: {
                title: "You have not selected any candidates",
                content:
                    "You have not made any selections. Your ballot will be cast as a blank ballot, which is a valid, deliberate choice and will be counted as such.",
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
            step1Title: "1. Make your selections",
            step1Description:
                "Pick your preferred candidates and make your selections in each contest as it appears. You can change your ballot anytime before casting your vote",
            step2Title: "2. Review your selections",
            step2Description:
                "When you’re happy with your selections, we’ll securely encrypt your ballot and show you a final review. You’ll also get a unique Tracking ID for reference",
            step3Title: "3. Cast your ballot",
            step3Description:
                "When you’re ready, cast your ballot so it’s officially recorded. Or choose to audit first to confirm it was correctly captured and encrypted",
        },
        reviewScreen: {
            acclamation: {
                title: "Decided by acclamation",
                helpDialog: {
                    title: "Information: Acclamation",
                    content:
                        "This screen shows what was decided by acclamation. Because no option could be selected, no ballot is cast and there is nothing to verify afterwards.",
                    ok: "OK",
                },
                description:
                    "Review what was decided by acclamation in this election. No ballot will be cast.",
                finishButton: "Finish",
            },
            title: "Review your ballot",
            description:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below, and to audit your ballot click the “<b>Audit Ballot</b>” button below.",
            descriptionNoAudit:
                "To make changes in your selections, click “<b>Edit ballot</b>” button, to confirm your selections, click “<b>Cast your ballot</b>” button below.",
            backButton: "Edit ballot",
            castBallotButton: "Cast ballot",
            auditButton: "Audit ballot",
            copyBallotId: "Copy tracking ID",
            ballotIdCopied: "Tracking ID copied",
            ballotIdCopyError: "Could not copy tracking ID",
            reviewScreenHelpDialog: {
                title: "About the review screen",
                content: "This screen lets you review your selections before casting your ballot",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "Your vote has not been cast",
                content:
                    "<p>This is your Tracking ID, but <b>your vote has not been cast yet</b>. If you try to track it now, nothing will appear.</p><p>We show the Tracking ID at this stage so you can audit the encrypted ballot before casting it.</p>",
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
            confirmCastBlankBallotDialog: {
                title: "Are you sure you want to cast a blank ballot?",
                content:
                    "You have not selected any candidates. After you confirm, your ballot will be cast as blank.",
                ok: "Yes, cast my blank ballot",
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
                    "The selection state for this election is missing. Please check your selections or contact support",
                NO_BALLOT_STYLE: "This ballot is not available. Please contact support",
                NO_AUDITABLE_BALLOT:
                    "There is no ballot available for audit. Please contact support",
                INCONSISTENT_HASH:
                    "There was an error related to the ballot hashing process. Tracking ID: {{ballotId}} is not consistent with auditable Ballot Hash: {{auditableBallotHash}}. Please report this issue to support.",
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
                CAST_VOTE_BallotIdMismatch: "The tracking ID does not match the cast vote.",
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
            blankBallot: "Blank ballot",
        },
        confirmationScreen: {
            acclamation: {
                title: "Decided by acclamation",
                description:
                    "Every contest in this election was decided by acclamation, so no ballot was cast and there is no tracking ID.",
                helpDialog: {
                    title: "Information: Acclamation",
                    content:
                        "Every contest in this election was decided by acclamation: its candidates are elected without a vote. Because no ballot was cast, there is no tracking ID, receipt or QR code to verify.",
                    ok: "OK",
                },
            },
            title: "Your vote has been cast",
            description:
                "Your ballot was cast successfully. Use the code below to verify that it was counted",
            blankBallot: {
                description: "Your ballot was cast blank, which is a valid, deliberate choice.",
            },
            ballotId: "Tracking ID",
            printButton: "Print",
            finishButton: "Finish",
            remainingElectionsError:
                "We couldn’t check whether you have more elections to vote in. Please retry.",
            retryButton: "Retry",
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
                title: "Tracking ID",
                content: "Code use is disabled in demo mode",
                ok: "OK",
            },
            ballotIdHelpDialog: {
                title: "About the Tracking ID",
                content:
                    "The Tracking ID is a unique code that lets you find your ballot in the ballot box. It does not contain any information about your selections.",
                ok: "OK",
            },
            ballotIdDemoHelpDialog: {
                title: "About the Tracking ID",
                content:
                    "The Tracking ID is a unique code that lets you find your ballot in the ballot box. It does not contain any information about your selections.",
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
                "your <b>Tracking ID</b> at the top of the screen and your encrypted ballot shown below",
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
                    "Welcome to the Voting Booth. This screen shows the ballots you can vote on. Ballots displayed in this list can be open to voting, scheduled, or closed. You will be able to access the ballot only if the voting period is open.",
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
            materialsGate: {
                instructions:
                    "You must read the <MaterialsLink>{{materialsTitle}}</MaterialsLink> before you can vote.",
            },
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write-in doesn't end on 0",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but the contest doesn't allow it",
                alert: "This selection will be counted as an invalid vote",
            },
            page: {
                oopsWithStatus: "Oops! {{status}}",
                oopsWithoutStatus: "Oops! Unexpected Error",
                somethingWrong: "Something went wrong.",
                certAuthFailedTitle: "Certificate Authentication Failed",
                certAuthFailedMessage:
                    "Your certificate could not be verified. Please check that you are using a valid voter certificate and try again.",
                invalidLoginHintParametersTitle: "Invalid voting link",
                invalidLoginHintParametersMessage:
                    "This voting link contains invalid login information. Request a new link and try again.",
            },
        },
        materials: {
            common: {
                label: "Support Materials",
                back: "Back to ballot list",
                close: "Close",
                preview: "Preview",
                download: "Download",
            },
            mandatory: {
                checkboxLabel: "I have read the Support Materials",
                continueButton: "Continue",
                error: "There was a problem recording your acknowledgment. Please try again.",
            },
        },
        ballotLocator: {
            title: "Ballot Tracking",
            titleResult: "Ballot Tracking",
            description: "Confirm your ballot was cast correctly",
            descriptionAudited:
                "This ballot was audited, so it was never cast. Check that the selections decoded below are the ones you made.",
            descriptionAuditedBrief: "This ballot was audited, so it was never cast.",
            locate: "Find your Ballot",
            locateAgain: "Find another Ballot",
            found: "Your tracking ID {{ballotId}} has been found",
            notFound: "Your tracking ID {{ballotId}} was not found",
            contentDesc: "This is your Ballot content: ",
            wrongFormatBallotId: "Invalid Tracking ID format",
            ballotIdNotFoundAtFilter:
                "Not found, check that your Tracking ID is correct and belongs to this user.",
            filterByBallotId: "Filter by Tracking ID",
            totalBallots: "Total Ballots: {{total}}",
            idHelp: "The short Tracking ID, like 30ff d314, or the full one",
            foundShort:
                "Found by its short ID. To be certain it is yours, compare the full Tracking ID with your receipt.",
            search: {
                title: "Check your ballot in the public record",
                intro: "Enter the Tracking ID from your confirmation screen or receipt. Your browser looks it up and checks the proofs itself.",
                find: "Find",
                invalid: "A Tracking ID has only digits and the letters a to f.",
                chips: {
                    short: "<b>Short</b> 30ff d314",
                    full: "<b>Full</b> 16 groups of four",
                    spaces: "Spaces optional",
                },
                features: {
                    cast: {
                        title: "Cast ballots",
                        text: "See that it is in the public record, confirmed by independent witnesses and anchored publicly.",
                    },
                    audited: {
                        title: "Audited ballots",
                        text: "See the selections it holds, decrypted in your browser, and check they are the ones you made.",
                    },
                    secret: {
                        title: "Your vote stays secret",
                        text: "A cast ballot stays encrypted. The tracker shows that it was counted, never how you voted.",
                    },
                },
            },
            matches: {
                title: "{{total}} ballots share the short ID <b>{{shortId}}</b>. Choose yours, or enter the full Tracking ID.",
                cast: "Cast {{time}}",
                audited: "Audited {{time}}",
            },
            audit: {
                title: "Your selections, decoded",
                note: "Your browser decrypted this audited ballot with the key the portal revealed. If these are not the selections you made, contact the election authority.",
                failed: {
                    title: "Audit failed",
                    text: "The ballot in the public record does not decrypt with the key the portal revealed. Do not cast from this device; tell the election authority.",
                },
                unchecked: {
                    title: "The audit could not be checked",
                    text: "This device could not check this audited ballot, so no selections are shown. Try again later, or check the ballot content in the ballot verifier.",
                },
            },
            result: {
                receipt: "Ballot receipt",
                idComment: "Use the short ID or the full one to find this ballot again.",
                fullId: "Full Tracking ID",
                copy: "Copy the full Tracking ID",
                copied: "Tracking ID copied",
                tag: {
                    found: "Found",
                    auditFailed: "Audit failed",
                },
                record: {
                    title: "Your ballot content",
                    subtitle: "Encrypted, as in the public record",
                },
                recordEntry: "Record entry",
                save: "Save receipt",
                sealLabel: "Ballot status: {{status}}",
                check: "Check it independently",
                caption: "Checked by your browser, with code anyone can inspect.",
                captionVerified: "Verified by your browser, with code anyone can inspect.",
                captionProblem: "Found by your browser, with code anyone can inspect.",
                late: "This usually means a hub is slow or offline; it is not evidence of tampering. Check again later, here or on any other device.",
                problem:
                    "The portal lists this ballot as cast, but the election’s public record has no entry for it. Your browser kept the evidence: the portal’s answer and the record’s signed checkpoint. Send it to the election authority or an observer.",
                problemOther:
                    "Signed proofs for this ballot contradict each other. It may be a fault or an attack. Your browser kept the evidence. Send it to the election authority or an observer.",
                saveEvidence: "Save evidence",
                howToReport: "How to report it",
                howToReportText:
                    "Save the evidence file and send it to the election authority or to an independent observer. They can check it on their own computers. It contains nothing about how you voted.",
                noReceipt:
                    "This device has no receipt for this ballot. Check it on the device you voted on, or check the receipt file in the ballot verifier.",
                receiptPending:
                    "This device keeps a receipt for this ballot, but it isn’t complete or couldn’t be checked just now. Try again later, or check the receipt file in the ballot verifier.",
                status: {
                    checking: {
                        head: "Checking…",
                        sub: "Your browser is looking the ballot up in the public record.",
                    },
                    recorded: {
                        head: "In the public record",
                        sub: "Your browser found the ballot. Checking its witnesses…",
                    },
                    witnessed: {
                        head: "Confirmed by witnesses",
                        sub: "Checking the public anchors…",
                        subAnchored_one:
                            "Anchored in public hubs. Where browsers check follows within {{count}} minute.",
                        subAnchored_other:
                            "Anchored in public hubs. Where browsers check follows within {{count}} minutes.",
                    },
                    verified: {
                        head: "Verified",
                        sub: "In the public record, confirmed by independent witnesses and anchored publicly.",
                    },
                    late: {
                        head: "Taking longer than expected",
                        sub: "In the public record and confirmed by witnesses, but its public anchors are late.",
                    },
                    problem: {
                        head: "Not in the public record",
                        sub: "Your browser could not find this ballot in the election’s public record.",
                    },
                    otherProblem: {
                        head: "A check failed",
                        sub: "Signed proofs for this ballot contradict each other. See the step that failed.",
                    },
                },
                steps: {
                    record: {
                        title: "Added to the public record",
                        done: "Entry #{{index}} in the election’s ballots log. Your browser checked its proof.",
                        failed: "No entry in the election’s ballots log has this Tracking ID.",
                        missing: "Missing",
                    },
                    witnesses: {
                        title: "Confirmed by witnesses",
                        done: "Independent witnesses signed a record that includes it.",
                        count: "{{signed}} of {{total}} · {{needed}} needed",
                        countOf: "{{signed}} of {{total}}",
                    },
                    hubs: {
                        title_one: "Anchored in {{count}} public hub",
                        title_other: "Anchored in {{count}} public hubs",
                        titleAny: "Anchored in public hubs",
                        done: "Shared logs also hold this record: {{hubs}}.",
                    },
                    browser: {
                        title: "Anchored where browsers check",
                        done: "Held by a stand-in hub until browsers check logs themselves.",
                    },
                    checking: "Your browser is checking…",
                    scheduled_one: "Within {{count}} minute. No need to wait.",
                    scheduled_other: "Within {{count}} minutes. No need to wait.",
                    by: "By {{time}}",
                    late: "Late",
                    lateHubs: "The hubs should have anchored it by {{time}}.",
                    lateOther: "This should have been done by {{time}}.",
                    waiting: "Waiting for the step above.",
                    skipped: "Not checked.",
                    failedStep: "Signed proofs for this step contradict each other.",
                    failedTime: "Problem",
                },
            },
            steps: {
                lookup: "Find your Ballot",
                result: "Result",
            },
            titleHelpDialog: {
                title: "About Ballot Tracking",
                content:
                    "This screen allows you to find your vote by using the Tracking ID to retrieve it. This procedure enables you to check that your ballot was correctly cast and that the recorded ballot matches the encrypted ballot you sent.",
                ok: "OK",
            },
            tabs: {
                logs: "Logs",
                ballotLocator: "Ballot Tracking",
            },
            column: {
                statement_kind: "Statement kind",
                statement_timestamp: "Statement Timestamp",
                ballot_id: "Tracking ID",
            },
        },
        vitrine: {
            ballotId: {
                label: "Tracking ID",
                full: "Full Tracking ID",
                yours: "Your Tracking ID:",
                copy: "Copy the full Tracking ID",
                copied: "Tracking ID copied",
                showFull: "Show the full Tracking ID",
            },
            status: {
                title: "Ballot status",
                cast: "Cast {{when}}",
                audited: "Audited {{when}}",
                ballotCast: "Ballot cast",
                aria: "Ballot status: {{state}}. Show details",
                ariaCast: "Ballot cast. Ballot status: {{state}}. Show details",
                tooltip: "{{state}} · Show details",
                tracker: "Open in the ballot tracker",
                save: "Save receipt",
                ok: "OK",
                tag: {
                    cast: "Cast",
                    notCast: "Not cast",
                    auditable: "Auditable",
                },
                state: {
                    notcast: "Not cast yet",
                    checking: "Checking…",
                    recorded: "In the public record",
                    witnessed: "Confirmed by witnesses",
                    verified: "Verified",
                    late: "Late",
                    problem: "Problem",
                    auditable: "Auditable",
                    auditfail: "Audit failed",
                },
                head: {
                    late: "Taking longer than expected",
                    mismatch: "The record does not match",
                    damaged: "The receipt is damaged",
                    noReceipt: "No receipt for your ballot",
                    unchecked: "Your ballot could not be checked",
                },
                sub: {
                    notcast:
                        "This ballot is encrypted on this device and nothing has been sent. Once you cast it, your browser checks that it reaches the public record.",
                    checking: "Your browser is checking that your ballot is in the public record.",
                    recorded:
                        "Your browser found your ballot in the public record. Witnesses confirm it next.",
                    witnessed:
                        "Independent witnesses confirmed the record. Public anchors follow within {{minutes}} minutes; no need to wait.",
                    verified:
                        "Your ballot is in the public record, confirmed by independent witnesses and anchored publicly.",
                    late: "Your ballot is in the public record and confirmed by witnesses, but its public anchors are late.",
                    lateStep:
                        "A step is taking longer than expected. This is not evidence of tampering, and your browser keeps checking.",
                    mismatch:
                        "The public record’s entry #{{index}} is not the one in this receipt.",
                    mismatchNoEntry: "The public record’s entry is not the one in this receipt.",
                    damaged:
                        "Part of the receipt or the record could not be read or verified. That is a data fault, not evidence of tampering. Your vote was cast.",
                    noReceipt:
                        "This website did not get a receipt for your ballot. Your vote was cast, and the election’s observers have been told.",
                    unchecked: "This website could not check the receipt. Your vote was cast.",
                    auditable:
                        "You audited this ballot, so it was never cast. It is in the public record, and the ballot tracker decrypts it to show the selections it holds.",
                    auditfail:
                        "The ballot in the public record does not decrypt with the key the portal revealed. Do not cast from this device; tell the election authority.",
                },
                step: {
                    record: {
                        title: "In the public record",
                        upcoming: "Your browser finds the ballot in the election’s public record.",
                    },
                    witnesses: {
                        title: "Confirmed by witnesses",
                        upcoming: "Independent witnesses sign a record that includes it.",
                    },
                    anchors: {
                        title: "Anchored publicly",
                        upcoming: "Public hubs, and the logs browsers check, hold that record.",
                    },
                },
                row: {
                    waiting: "Waiting for the step above.",
                    checking: "Your browser is checking…",
                    anchoring:
                        "Public hubs, then where browsers check: within {{minutes}} minutes.",
                    recorded: "Your browser checked its proof.",
                    recordedEntry: "Entry #{{index}}. Your browser checked its proof.",
                    witnessed:
                        "Enough of the policy’s independent witnesses ({{count}}) signed a record that includes it.",
                    anchoredWaict: "In the public hubs ({{count}}), and where browsers check.",
                    anchoredBootstrap:
                        "In the public hubs ({{count}}), and where browsers check, through a stand-in hub for now.",
                    anchoredHubs: "In the public hubs ({{count}}).",
                    lateNow: "It should have been done by now.",
                    lateStep: "It should have been done by {{time}}.",
                    lateAnchors: "The hubs should have anchored it by {{time}}.",
                    mismatch: "Entry #{{index}} in the record differs from your receipt.",
                    mismatchNoEntry: "An entry in the record differs from your receipt.",
                    contradiction: "Its signed proofs contradict each other.",
                    damaged: "Its proof is damaged and could not be verified.",
                    noReceipt: "This website did not get a receipt.",
                    unchecked: "This website could not check the receipt.",
                    skipped: "Not checked, because the step above could not be completed.",
                    skippedMismatch: "Not checked, because the record does not match.",
                },
                word: {
                    withinSeconds: "Within seconds",
                    withinMinutes: "Within {{minutes}} min",
                    checking: "Checking…",
                    late: "Late",
                    mismatch: "Mismatch",
                    damaged: "Damaged",
                    noReceipt: "No receipt",
                    unchecked: "Not checked",
                    skipped: "—",
                },
                callout: {
                    late: "This usually means a hub is slow or offline; it is not evidence of tampering.",
                    lateKept:
                        "This device checks again each time you come back, and the ballot tracker can check from any device.",
                    lateTracker: "The ballot tracker can check from any device.",
                    evidence:
                        "Your browser kept the evidence: this receipt and the record’s signed entry. Send it to the election authority or an observer; anyone can check it.",
                    saveEvidence: "Save evidence",
                },
                caption: {
                    notcast:
                        "Nothing is checked until you cast it. Then this check mark shows how far your ballot has been verified.",
                    checking:
                        "Your browser is checking the public record, with code anyone can inspect.",
                    problem: "Found by your browser, with code anyone can inspect.",
                    auditable: "Decrypted by your browser, with code anyone can inspect.",
                    verified: "Verified by your browser, with code anyone can inspect.",
                    checked: "Checked by your browser, with code anyone can inspect.",
                    kept: "This device keeps the receipt and checks it again on each visit.",
                },
            },
            booth: {
                title: "Your ballot can't be prepared yet",
                explanation:
                    "Before it encrypts your ballot, this website checks that the election's settings are the ones published in the election's public record. Your ballot hasn't been encrypted or sent, and your choices are still here.",
                pending:
                    "The published settings aren't available yet. Please try again in a moment.",
                invalid: "The published settings couldn't be read. Please try again in a moment.",
                failed: "The settings don't match the published ones. The election's observers have been told. Please contact support.",
                retry: "Try again",
                close: "Close",
            },
            record: {
                json: "JSON",
                text: "Text",
                fields_one: "{{count}} field",
                fields_other: "{{count}} fields",
                items_one: "{{count}} item",
                items_other: "{{count}} items",
                size: {
                    bytes: "{{value}} B",
                    kilobytes: "{{value}} KB",
                },
                copy: "Copy {{file}}",
                copied: "Copied",
                download: "Download {{file}}",
            },
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
