// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const englishTranslation = {
    translations: {
        404: {
            title: "Page not found",
            subtitle: "The page you are looking for does not exist",
        },
        homeScreen: {
            step1: "Step 1: Import your ballot",
            description1:
                "To continue, please import the encrypted ballot data  provided in the Voting Portal:",
            importBallotHelpDialog: {
                title: "Information: Import your ballot",
                ok: "OK",
                content:
                    "To continue, please import the encrypted ballot data  provided in the Voting Portal.",
            },
            step2: "Step 2: Insert your tracking ID",
            description2:
                "Paste the full Tracking ID from the Voting Portal: the copy button next to your Tracking ID copies it.",
            ballotIdHelpDialog: {
                title: "Information: Your tracking ID",
                ok: "OK",
                content:
                    "Paste the full Tracking ID provided in the Voting Portal. The copy button next to it copies it.",
            },
            startButton: "Browse file",
            dragDropOption: "Or drag and drop it here",
            importErrorDescription:
                "There was a problem importing the auditable ballot. Did you choose  the right file?",
            importErrorMoreInfo: "More info",
            importErrorTitle: "Error",
            ciphertextErrorTitle: "Ballot verification failed",
            ciphertextErrorDescription:
                "The ciphertext in this auditable ballot is not the encryption of the plaintext and randomness it contains. The ballot cannot be trusted.",
            verificationErrorTitle: "Ballot verification failed",
            unpublishedStyleErrorDescription:
                "This auditable ballot does not match any ballot published for this election event, so it cannot be verified.",
            unavailableStyleErrorDescription:
                "The ballots published for this election event could not be loaded, so this auditable ballot cannot be verified yet. Please try again.",
            styleErrorDescription:
                "The ballot style in this auditable ballot is not the one published for the election: its public key or its contests and candidates differ, or it is not published. The ballot cannot be trusted.",
            legacyBallotWarning:
                "This auditable ballot uses an older ballot format whose Ballot ID does not cover the ballot style. Its Ballot ID cannot confirm that the ballot was encrypted with the published election key.",
            useSampleLink: "Use a sample ballot",
            nextButton: "Next",
            fileUploaded: "Uploaded",
            receiptTitle: "Transparency receipt",
            receiptDescription:
                "This election publishes a receipt for every ballot cast. Check yours: that it is in the public record, unchanged and the same for everyone.",
            receiptButton: "Check a receipt",
        },
        confirmationScreen: {
            ballotIdTitle: "Tracking ID",
            ballotIdDescription:
                "Below, the system shows the Tracking ID of the decoded ballot, and the one you provided.",
            ballotIdError: "Doesn’t match the decoded Tracking ID",
            decodedBallotId: "Decoded Tracking ID",
            decodedBallotIdHelpDialog: {
                title: "Information: Decoded Tracking ID",
                ok: "OK",
                content:
                    "This is the Tracking ID read from decoding the Auditable Ballot file you provided.",
            },
            yourBallotId: "The Tracking ID you provided",
            userBallotIdHelpDialog: {
                title: "Information: The Tracking ID you provided",
                ok: "OK",
                content:
                    "This is the Tracking ID you typed in the previous step and that you collected from the Voting Booth.",
            },
            backButton: "Back",
            printButton: "Print",
            finishButton: "Verified",
            verifySelectionsTitle: "Verify your ballot selections",
            verifySelectionsDescription:
                "The following ballot selections have been decoded from the ballot you imported. Please review them and ensure that they match the selections you made in the Voting Portal.  If your selections don't match, please contact the election authorities...",
            verifySelectionsHelpDialog: {
                title: "Information: Verify your ballot selections",
                ok: "OK",
                content:
                    "The following ballot selections have been decoded from the ballot you imported. Please review them and ensure that they match the selections you made in the Voting Portal.  If your selections don't match, please contact the election authorities...",
            },
            markedInvalid: "Ballot explicitly marked invalid",
            points_one: "({{count}} Point)",
            // `_many` is selected only in es/cat/fr, for exact multiples of a million. English never
            // selects it, but the other bundles are typed `TranslationType = typeof englishTranslation`,
            // so the key has to be declared here before they can carry it.
            points_many: "({{count}} Points)",
            points_other: "({{count}} Points)",
            contestNotFound: "Contest not found: {{contestId}}",
            declineToVote: "Declined to vote",
            blankBallot: "Blank ballot",
            acclamationDescription:
                "This contest was decided by acclamation. Its candidates are elected without a vote, so no option can be selected and no vote is recorded for it.",
        },
        receiptScreen: {
            title: "Check a transparency receipt",
            description:
                "A transparency receipt shows that your ballot is in the election’s public record, unchanged and the same for everyone. This page checks it in your browser, against the trust policy that the election authority published.",
            loading: "Loading the receipt checker…",
            loadFailed: "The receipt checker couldn't load. Reload the page to try again.",
            receipt: {
                title: "Your receipt",
                description:
                    "Open the receipt file you saved after voting, or paste the receipt, its link or its reference (the text of its QR code).",
                fileFormat: "Receipt files end in .vitrine-receipt",
                pasteLabel: "Receipt, link or reference",
                pastePlaceholder: "Paste it here",
                open: "Open",
                another: "Check another receipt",
                problems: {
                    "empty": "Paste a receipt, its link or its reference first.",
                    "too-large": "This is too large to be a receipt.",
                    "bad-link":
                        "This link doesn't hold a receipt that can be read. Open the receipt file instead.",
                    "bad-reference":
                        "This reference is incomplete or damaged. Copy it again, or open the receipt file.",
                    "bad-chain": "This receipt is damaged: it can't be read.",
                    "not-https":
                        "The link says the policy is at an address that isn't secure (https), so it isn't used.",
                    "wrong-box": "This is a trust policy, not a receipt. Open the receipt first.",
                    "unrecognised": "This isn't a receipt, a receipt link or a receipt reference.",
                    "unreadable": "The file couldn't be read.",
                },
            },
            policy: {
                title: "Trust policy",
                description:
                    "Receipts are checked against the trust policy that the election authority published. Give it here, as its text or its https address, from the official election notice.",
                fromLink:
                    "The receipt's link says where its trust policy is. Before you pin it on this device, compare its words and ID with the official election notice.",
                inputLabel: "Trust policy or its address",
                inputPlaceholder: "Paste the policy, or its https address",
                load: "Load the policy",
                openFile: "Open a policy file",
                loading: "Loading the trust policy…",
                pin: "They match: pin this policy",
                reject: "They don't match",
                rejected:
                    "Don't use this policy. Tell the election authority, and give the policy from the official election notice.",
                pinned: "Checked against the trust policy you pinned on this device.",
                unpin: "Forget this policy",
                draft: "This is a draft of the policy, not the sealed policy, so it can't be pinned. Its draft words:",
                invalid: "This isn't a valid trust policy.",
                otherPolicy: "This trust policy is valid, but the receipt is for another one.",
                ambiguous:
                    "More than one policy you pinned matches this reference, so it isn't used. Open the receipt file instead.",
                problems: {
                    "empty": "Paste the trust policy or its address first.",
                    "too-large": "This is too large to be a trust policy.",
                    "not-https": "Only addresses that start with https are used.",
                    "wrong-box": "This is a receipt, not a trust policy.",
                    "unrecognised": "This isn't a trust policy or its address.",
                    "unreadable": "The file couldn't be read.",
                },
                fetchProblems: {
                    "not-https": "Only addresses that start with https are used.",
                    "unreachable":
                        "The trust policy couldn't be loaded. Check the address, or paste the policy instead.",
                    "status": "The server didn't return the trust policy (status {{status}}).",
                    "too-large": "This is too large to be a trust policy.",
                    "not-text": "The address didn't return a trust policy.",
                },
            },
            ballot: {
                title: "Your ballot",
                description:
                    "The receipt shows as yours only for the ballot you cast. Paste the full Tracking ID that the voting portal showed you.",
                election: "Election ID",
                electionPlaceholder: "Choose the election",
                event: "Election event ID",
                check: "Check the receipt",
                change: "Change the tracking ID",
                lookedFor: "Tracking ID looked for",
                fullId: "Full Tracking ID",
                stage: "Step of the ballot",
                stages: {received: "Received", cast: "Cast", audited: "Audited"},
            },
            result: {
                title: "Result",
                rounds: {
                    log: "Checked with the election’s logs. Within the next half hour, this page also asks the logs’ mirrors, the anchor hubs and the monitors, at a random moment, so that they can’t tell when you checked.",
                    full: "Checked with the election’s logs, their mirrors, the anchor hubs and the monitors.",
                    offline:
                        "This election's logs are private, so the receipt was checked with what it holds.",
                },
                checkEverything: "Check everything now",
                checkAgain: "Check again",
                retry: "Try again",
                problems: {
                    "policy-mismatch": "This reference is for another trust policy.",
                    "unknown-log": "The trust policy has no log of this name.",
                    "quiet-period":
                        "The log isn't serving proofs during its quiet period. Try again after {{time}}.",
                    "not-found":
                        "The log doesn't have this entry yet. If you voted recently, try again later.",
                    "entry-mismatch":
                        "The log's entry at this position isn't the one in the reference.",
                    "private-reference":
                        "This election's logs are private, so a reference can't be checked here. Open the receipt file instead.",
                    "failed": "The receipt couldn't be checked. Try again.",
                },
                evidenceSaved:
                    "The receipt file was saved. It holds the signed objects that contradict each other: an auditor can check it, and build the evidence file from it, with the Vitrine verifier.",
                advice: {
                    failed: "A check failed: signed data contradicts itself or the trust policy. Save the evidence, and give it with your receipt file to the election authority and to the observers that the trust policy names.",
                    late: "Something that should be there by now isn’t there yet. It may only be slow: check again later, and if it stays late, tell the election authority. Keep your receipt file.",
                },
            },
            status: {
                checking: {
                    head: "Checking…",
                    sub: "Your browser is checking that your ballot is in the public record.",
                },
                recorded: {
                    head: "In the public record",
                    sub: "Your browser found your ballot in the public record. Witnesses confirm it next.",
                },
                witnessed: {
                    head: "Confirmed by witnesses",
                    sub: "Independent witnesses confirmed the record. The public hubs are asked next, within half an hour.",
                    subLater:
                        "Independent witnesses confirmed the record. Public anchors follow within 10 minutes; no need to wait.",
                },
                verified: {
                    head: "Verified",
                    sub: "Your ballot is in the public record, confirmed by independent witnesses and anchored publicly.",
                },
                late: {
                    head: "Taking longer than expected",
                    sub: "Your ballot is in the public record and confirmed by witnesses, but its public anchors are late.",
                    subOther: "Some of the steps below are taking longer than expected.",
                },
                problem: {
                    head: "The record does not match",
                    sub: "The public record’s entry #{{index}} is not the one in this receipt.",
                    subOther:
                        "The public record, this receipt and your Tracking ID don’t all agree.",
                },
                damaged: {
                    head: "Couldn’t be checked",
                    sub: "Some data looks damaged: the receipt or the server’s answer. This doesn’t show that anyone cheated.",
                },
                evidence:
                    "Your browser kept the evidence: this receipt and the record’s signed entry. Send it to the election authority or an observer; anyone can check it.",
                saveEvidence: "Save evidence",
                howToReport: "How to report it",
                captionChecking:
                    "Your browser is checking the public record, with code anyone can inspect.",
                captionChecked: "Checked by your browser, with code anyone can inspect.",
                captionProblem: "Found by your browser, with code anyone can inspect.",
                rows: {
                    record: "In the public record",
                    witnesses: "Confirmed by witnesses",
                    anchors: "Anchored publicly",
                    recordDone: "Entry #{{index}}. Your browser checked its proof.",
                    recordDoneNoIndex: "Your browser checked its proof.",
                    witnessesDone:
                        "{{signed}} of {{total}} independent witnesses signed; the policy needs {{needed}}.",
                    witnessesDoneAny: "Independent witnesses signed it.",
                    hubs_one: "In {{count}} public hub",
                    hubs_many: "In {{count}} public hubs",
                    hubs_other: "In {{count}} public hubs",
                    browser: ", and where browsers check",
                    standIn: ", through a stand-in hub for now",
                    unused: "Not used by this election’s policy.",
                    checking: "Your browser is checking…",
                    anchorsActive: "Asked within the next half hour, at a random moment.",
                    anchorsPending: "Not anchored yet. Check again in a few minutes.",
                    waiting: "Waiting for the step above.",
                    lateAnchors: "The hubs should have anchored it by {{time}}.",
                    lateOther: "It should be done by now.",
                    recordFailed: "Entry #{{index}} in the record differs from your receipt.",
                    failed: "It doesn’t agree with your receipt and Tracking ID.",
                    damaged: "Couldn’t be checked: the data looks damaged.",
                    skipped: "Not checked, because the record does not match.",
                    skippedDamaged: "Not checked, because an earlier step couldn’t be checked.",
                    statusChecking: "Checking…",
                    withinSeconds: "Within seconds",
                    within10: "Within 10 min",
                    within30: "Within 30 min",
                    statusLate: "Late",
                    statusMismatch: "Mismatch",
                    statusDamaged: "Damaged",
                },
            },
        },
        footer: {
            poweredBy: "Powered by <1></1>",
        },
        errors: {
            encoding: {
                notEnoughChoices: "Not enough choices to decode",
                writeInChoiceOutOfRange: "Write-in choice out of range: {{index}}",
                writeInNotEndInZero: "Write in doesn't end on 0",
                bytesToUtf8Conversion:
                    "Error converting write-in from bytes to UTF-8 string: {{errorMessage}}",
                ballotTooLarge: "Ballot larger than expected",
            },
            explicit: {
                notAllowed: "Ballot marked explicitly invalid but the contest doesn't allow it",
            },
        },
        ballotIdField: {
            label: "Tracking ID",
            placeholder: "Paste your full Tracking ID",
            help: "Spaces are optional.",
            shortError:
                "That’s the short ID. Paste the full Tracking ID: the check compares all of it.",
            formatError:
                "Check the Tracking ID: it has 64 characters, digits and the letters a to f.",
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
