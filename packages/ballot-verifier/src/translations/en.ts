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
                "This auditable ballot uses an older ballot format whose Tracking ID does not cover the ballot style. Its Tracking ID cannot confirm that the ballot was encrypted with the published election key.",
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
                    "The receipt's link says where its trust policy is. Before you pin it on this device, compare the first four groups of its policy ID with the official election notice.",
                compareGroups: "Compare the first four groups, in bold, with the notice. Tools compare all 13.",
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
                draft: "This is a draft of the policy, not the sealed policy, so it can't be pinned. Its draft ID:",
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
        auditedScreen: {
            title: "Ballot",
            tag: "Audited",
            loading: "Checking the ballot in the public record…",
            retry: "Try again",
            header: {
                strip: {
                    withOperator:
                        "A transparency hub, run by {{operator}}. It isn’t the election’s website, and never asks you to vote, sign in or pay.",
                    noOperator:
                        "A transparency hub. It isn’t the election’s website, and never asks you to vote, sign in or pay.",
                },
                subline: {
                    withOperator: "Run by {{operator}} · a Vitrine hub",
                    noOperator: "A Vitrine hub",
                },
                unnamed: "Transparency hub",
            },
            trackingId: {
                full: "Full Tracking ID",
                copy: "Copy the full Tracking ID",
                copied: "Tracking ID copied",
                copyText: "Copy",
                copiedText: "Copied",
                selected: "Selected. Press Ctrl+C, or ⌘C on a Mac, to copy it.",
            },
            checks: {
                title: "Checked in your browser",
                record: {
                    title: "In the public record",
                    text: "Entry #{{index}}, audited at {{time}}. Never cast.",
                    waiting: "Looking for its audit in the election’s log.",
                    failed: "The log’s proof of this audit doesn’t check out.",
                },
                secret: {
                    title: "Secret matches the ballot",
                    text: "Re-encrypting with it gives the same ballot, byte for byte.",
                    waiting: "Checked once the record is.",
                    failed: "Re-encrypting with it doesn’t give this ballot.",
                },
                decoded: {
                    title: "Decoded",
                    text: "The choices on the left, from the election’s published ballot design.",
                    waiting: "Only once the secret matches.",
                    failed: "It couldn’t be decoded.",
                },
            },
            note: {
                named: "With code anyone can inspect. {{hub}} only stores and serves it, and never asks what you chose. Your answer stays in this browser.",
                unnamed:
                    "With code anyone can inspect. This hub only stores and serves it, and never asks what you chose. Your answer stays in this browser.",
            },
            states: {
                "bad-link": {
                    title: "This link isn’t complete",
                    text: "It needs the ballot’s full Tracking ID and the election’s policy ID. Open it again from the voting site.",
                },
                "no-policy": {
                    title: "The election’s policy couldn’t be found",
                    text: "This page checks a ballot only with the policy the link names, and couldn’t get it from the election. Try again later.",
                },
                "other-election": {
                    title: "This link is for another election",
                    text: "The policy the link names doesn’t cover this election, so the ballot can’t be checked with it.",
                },
                "not-found": {
                    title: "No ballot with this Tracking ID",
                    text: "The election’s public record has no ballot with this Tracking ID. Check that the link is complete, or try again in a minute.",
                },
                "cast": {
                    title: "This ballot was cast, not audited",
                    text: "Only an audited ballot can be decoded. A cast ballot stays encrypted, so no one sees how it was voted, here or anywhere else.",
                },
                "pending": {
                    title: "This ballot isn’t audited yet",
                    text: "The public record has this ballot, but not its audit yet. Try again in a minute.",
                },
                "no-content": {
                    title: "The audited ballot isn’t published yet",
                    text: "The public record has this ballot’s audit, but not the ballot and its secret, so it can’t be decoded here yet. Try again later.",
                },
                "unavailable": {
                    title: "The ballot couldn’t be checked",
                    text: "This page couldn’t reach the election’s public record. Try again later.",
                },
                "record-failed": {
                    title: "The record doesn’t prove this audit",
                    text: "The election’s log gave a proof of this ballot’s audit that doesn’t check out, so nothing is decoded. Tell the election authority.",
                },
                "mismatch": {
                    title: "Audit failed",
                    text: "The ballot in the public record isn’t the one its Tracking ID names, or the secret the portal published doesn’t reproduce it. Don’t vote on the device you audited from, and tell the election authority.",
                },
                "unchecked": {
                    title: "The audit could not be checked",
                    text: "This page couldn’t check this audited ballot, so no choices are shown. Try again later, or on another device.",
                },
            },
            compare: {
                kicker: "Ballot audit",
                title: "Check that your ballot was encrypted as you chose",
                note: "This is the ballot you audited, decoded by your browser. Only you know what you chose, so only you can do this check.",
                steps: {
                    label: "How the check works",
                    encrypted: {
                        title: "Encrypted on your device",
                        text: "The portal encrypted your choices and gave the ballot its Tracking ID, before you chose to audit it.",
                    },
                    decoded: {
                        title: "Decoded in your browser",
                        text: "The ballot and the secret the portal published both come from the public record. This page decoded one with the other.",
                    },
                    compare: {
                        title: "You compare",
                        text: "Look at the choices below. Are they the ones you made?",
                    },
                },
                choices: "Choices decoded from this ballot",
                question: "Do these match what you chose?",
                yes: "Yes, they match",
                no: "No, something’s different",
                change: "Change my answer",
                match: {
                    title: "Your ballot was encrypted as you chose",
                    text: "This audited ballot is now spent: it was never cast and will never be counted. Go back and vote. Your vote gets a new Tracking ID, and you can audit again first if you like.",
                    back: "Back to the voting site",
                },
                mismatch: {
                    title: "Don’t vote on this device",
                    text: "What was encrypted isn’t what you chose, so this device or the voting portal may have been tampered with. Your browser kept the evidence: the encrypted ballot from the public record, the secret the portal revealed, and what it decodes to.",
                    report: "Report it to the election authority",
                    reportNoContact: "Report it to the election authority.",
                    fromPolicy: "From the election’s published policy, not from this page.",
                    fromSettings: "From the election’s settings.",
                    then: "Then vote from another device.",
                    save: "Save evidence",
                    email: "Email the authority",
                    emailSubject: "My audited ballot does not decode to my choices",
                    emailBody:
                        "I audited a ballot in this election and the choices my browser decoded are not the ones I made.\n\nTracking ID: {{id}}\n\nI have saved the evidence as a file and can send it.",
                },
                how: "How does this work?",
                faq: {
                    why: {
                        question: "Why audit?",
                        answer: "Your vote is encrypted on your device, so no one can read it, not even you. Auditing lets you check that the portal encrypted what you chose before you vote for real.",
                    },
                    catch: {
                        question: "Why does it catch cheating?",
                        answer: "The portal encrypts before you decide to audit, so it can’t know which ballots will be checked. If it changed votes, every voter who audits could catch it.",
                    },
                    spent: {
                        question: "Why can’t this ballot count?",
                        answer: "With its secret revealed, the ballot shows how you voted. To keep your vote secret, an audited ballot is never cast.",
                    },
                    differ: {
                        question: "What if they don’t match?",
                        answer: "Don’t vote on that device. Save the evidence and report it to the election authority, whose contact comes from the election’s policy. Then vote from another device.",
                    },
                    tip: "For the strongest check, open this page on a different device from the one you voted on, such as your phone. A device that changed your ballot could also change what it shows you here.",
                },
            },
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
