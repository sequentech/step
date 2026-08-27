// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const englishTranslation = {
    translations: {
        language: "English",
        footer: {
            poweredBy: "Powered by <1></1>",
        },
        version: {
            header: "Version:",
        },
        hash: {
            header: "Hash:",
        },
        logout: {
            buttonText: "Logout",
            modal: {
                title: "Are you sure you want to logout?",
                content: "You are about to close this application.",
                ok: "OK",
                close: "Close",
            },
        },
        header: {
            profile: "Profile",
            welcome: "Welcome,<br><span>{{name}}</span>",
            session: {
                title: "Your session is going to expire.",
                timeLeft: "You have {{time}} left.",
                timeLeftMinutesAndSeconds: "{{timeLeftInMinutes}} minutes and {{time}} seconds",
                timeLeftSeconds: "{{timeLeft}} seconds",
            },
        },
        resultsPortal: {
            pageTitle: "Election Results",
            publishedResultsDescription: "Published results for this election event.",
            resultsAndParticipationTitle: "Results & Participation",
            electionsTitle: "Elections",
            contestsTitle: "Contests",
            areasTitle: "Areas",
            globalArea: "Global",
            noResultsForSelection: "No results are available for this selection.",
            version: "Version {{version}}",
            publicAccess: "Public access",
            signedInAccess: "Signed-in access",
            published: "Published",
            notPublishedYet: "Not published yet",
            position: "{{count}} position",
            position_plural: "{{count}} positions",
            fallbackElectionName: "Election",
            fallbackContestName: "Contest {{contestId}}",
            state: {
                unexpectedErrorTitle: "Unexpected error",
                loadErrorMessage:
                    "We could not load results right now. Please try again in a few minutes.",
                signInErrorMessage:
                    "We could not complete sign-in for results right now. Please try again in a few minutes.",
                signInRequiredTitle: "Sign in required",
                signInRequiredMessage:
                    "Please sign in with your voter account to view these results.",
                notPublishedTitle: "Results not published yet",
                notPublishedMessage:
                    "Results are not available at this time. Please check back later.",
            },
            summary: {
                title: "General information",
                ariaLabel: "General results information",
                election: "Election",
                eligibleVoters: "Eligible voters",
                totalVotesCounted: "Total votes counted",
                validVotes: "Valid votes",
                participation: "Participation",
                totalBlankBallots: "Total blank ballots",
            },
            resultsAndParticipation: {
                participationSummary: "Participation Summary",
                candidateResults: "Candidate Results",
                total: "Total",
                turnout: "%",
                eligibleCensus: "Eligible Voters",
                totalAuditableVotes: "Total Auditable Votes",
                totalVotesCounted: "Total Votes Counted",
                totalValidVotes: "Total Valid Votes",
                totalInvalidVotes: "Total Invalid Votes",
                explicitInvalidVotes: "Explicitly Invalid Votes",
                implicitInvalidVotes: "Implicitly Invalid Votes",
                blankVotes: "Blank Votes",
                explicitBlankVotes: "Explicit Blank Votes",
                implicitBlankVotes: "Implicit Blank Votes",
                blankVotesChart: "Blank Votes",
                weight: "Weight",
                options: "Options",
                castVotes: "Number of Votes",
                castVotesPercent: "Percent of Votes",
                winningPosition: "Winning position",
                votesForCandidates: "Votes For Candidates",
                invalidVotes: "Invalid Votes",
                nonVoters: "Non Voters",
                others: "Others",
                candidate: "Candidate",
                round: "Round",
                winner: "Winner",
                eliminated: "Eliminated",
                empty: "No items",
                previousRounds: "Navigate to previous rounds",
                nextRounds: "Navigate to next rounds",
                participationByChannel: "Participation by channel",
                channel: "Channel",
                channelOnline: "Online",
                channelKiosk: "Kiosk",
                channelEarlyVoting: "Early voting",
                channelTelephone: "Telephone",
                channelPaper: "Paper",
                channelPostal: "Postal",
                channelInPerson: "In person",
            },
        },
    },
}

export type TranslationType = typeof englishTranslation

export default englishTranslation
