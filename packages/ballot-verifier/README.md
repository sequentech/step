<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# ballot-verifier

Sequent cast-as-intended verifier. It allows a voter to audit an (spoiled)
ballot. `ballot-verifier` implements the 'cast or cancel' procedure described on
the paper [Ballot Casting Assurance via Voter-Initiated Poll Station Auditing](https://www.usenix.org/legacy/event/evt07/tech/full_papers/benaloh/benaloh.pdf)
by Josh Benaloh.

A transparency hub can run it to check an audited ballot by its Tracking ID:
`tenant/<t>/event/<e>/election/<el>/audited#id=<Tracking ID>&policy=sha256:<hex>`
opens the ballot from the election's public record, with no login and no
upload, and asks the voter to compare the decoded choices. `HUB_NAME` and
`VOTING_PORTAL_URL` in `global-settings.json` name the hub and the voting site.
See [A hub's ballot verifier](../../docs/docusaurus/docs/07-developers/14-vitrine.md#a-hubs-ballot-verifier).
The upload flow stays for events without transparency.

## Storybook

To run storybook, execute `yarn storybook`, then open `localhost:6006`. 

## Unit tests

See the [testing guide](../../docs/docusaurus/docs/07-developers/03-development-environment/testing/ballot-verifier.md)
for commands, fixtures and coverage scope.

The optional BrowserStack smoke test generates and verifies a new wbraid sample
ballot through the current portal controls. Point `BALLOT_VERIFIER_URL` at the
configured verifier's tenant/event start route with `?lang=en`, set the
BrowserStack credentials, and run `npm run local` from `browserstack/`. Published
ballot audit, tampering and tracker checks use the fixture-backed Playwright
journeys described in the testing guide.
