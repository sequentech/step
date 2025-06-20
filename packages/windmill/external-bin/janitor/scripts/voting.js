// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Automated Voting Test Script for Nightwatch.js
 *
 * This script simulates user voting sessions for load or integration testing of
 * the Sequent voting platform.
 *
 * Usage:
 *   export ENV_NAME='dev'
 *   export ELECTION_EVENT_ID='e14a57a3-0c54-41d9-bceb-89a2c2c206f3'
 *   export NUMBER_OF_VOTES='4096'
 * 
 *   # Optional environment variables:
 *   export TENANT_ID='90505c8a-23a9-4cdf-a26b-4e19f6a097d5'
 *   export VOTING_PASSWORD='User1234567!'
 *   export USERNAME_PATTERN='testsequent2025+{n}@mailinator.com'
 *   export SAVE_SCREENSHOTS='false'
 * 
 *   # Run the script with Nightwatch:
 *   npx nightwatch voting.js --env chrome
 *
 * Environment variables:
 *   ENV_NAME            - (default: 'dev') Environment name for the voting URL
 *   TENANT_ID           - (default: demo UUID) Tenant UUID
 *   ELECTION_EVENT_ID   - (default: demo UUID) Election event UUID
 *   VOTING_PASSWORD     - (default: 'User1234567!') Password for all users
 *   NUMBER_OF_VOTES     - (default: 4096) Number of unique users to simulate
 *   USERNAME_PATTERN    - (default: 'testsequent2025+{n}@mailinator.com') 
 *                         Username pattern, {n} will be replaced with a random
 *                         number
 *   SAVE_SCREENSHOTS    - (default: 'false') Whether to save screenshots
 *
 * The script will run in an infinite loop, simulating a new cast vote each
 * iteration.
 *
 * Screenshots are saved for each major step for debugging and audit purposes, 
 * in case it is enabled.
 */

module.exports = {
    'Automated Voting Test': function (browser) {
        // Use env vars for flexibility in execution and to avoid hardcoding
        // sensitive data
        const env_name = process.env.ENV_NAME || 'dev';
        const tenant_id = process.env.TENANT_ID || '90505c8a-23a9-4cdf-a26b-4e19f6a097d5';
        const election_event_id = process.env.ELECTION_EVENT_ID || 'e14a57a3-0c54-41d9-bceb-89a2c2c206f3';
        const url = `https://voting-${env_name}.sequent.vote/tenant/${tenant_id}/event/${election_event_id}/login`;
        const password = process.env.VOTING_PASSWORD || "User1234567!";
        const numberOfVotes = parseInt(process.env.NUMBER_OF_VOTES, 10) || 4096;
        const usernamePattern = process.env.USERNAME_PATTERN || 'testsequent2025+{n}@mailinator.com';
        const saveScreenshots = (process.env.SAVE_SCREENSHOTS || 'false').toLowerCase() !== 'false';

        // Username pattern is parameterized
        function getUsername(numberOfVotes) {
            // Using a random number ensures we hit a wide range of test users
            // and avoid hardcoding patterns
            const randomNumber = Math.floor(Math.random() * numberOfVotes) + 1;
            return usernamePattern.replace('{n}', randomNumber);
        }

        // Helper to optionally save screenshots
        function maybeScreenshot(path) {
            if (saveScreenshots) {
                browser.saveScreenshot(path);
            }
            return browser;
        }

        // Recursively call vote() to simulate continuous load; this is
        // intentional to generate a high load on the voting system
        function vote(iteration) {
            const username = getUsername(numberOfVotes);
            console.log(`Starting iteration ${iteration} for: ${username}`);
            
            browser
                // Always start from the login page to ensure a clean session
                .url(url)
                .waitForElementVisible('body', 20000)
                // Screenshots are optional for performance or disk space
                // reasons
                .perform(() => maybeScreenshot(`screenshots/login(${iteration}).png`))
                // Use unique credentials per iteration to ensure enough total
                // votes are captured; please ensure revoting is allowed because
                // this script will sometimes vote multiple times with the same
                // user
                .setValue('input[name=username]', username)
                .setValue('input[name=password]', password)
                .pause(500)
                .click('#kc-login')
                .pause(2500)
                .perform(() => maybeScreenshot(`screenshots/ballot_list_load${iteration}.png`))
                // Wait for the voting UI to load; this ensures the test doesn't
                // race ahead
                .waitForElementVisible("button.click-to-vote-button", 20000)
                .perform(() => maybeScreenshot(`screenshots/votingScreen${iteration}.png`))
                .click("button.click-to-vote-button")
                .pause(500)
                .waitForElementVisible("button.start-voting-button", 20000)
                .click("button.start-voting-button")
                .pause(1000)
                // Use browser.execute to dynamically discover contests and
                // candidates; this makes the test robust to ballot changes
                .execute(function () {
                    const contests = document.querySelectorAll('div[class^="contest-"]');
                    return Array.from(contests).map((contest, contestIndex) => {
                        const titleElement = contest.querySelector('h5[data-max]');
                        const maxVotes = parseInt(titleElement.getAttribute('data-max'), 10);
                        const candidateCheckboxes = contest.querySelectorAll('input[type="checkbox"][aria-label]');
                        return {
                            contestIndex,
                            maxVotes,
                            candidates: Array.from(candidateCheckboxes).map(checkbox => ({
                                name: checkbox.getAttribute('aria-label'),
                                selector: `input[aria-label="${checkbox.getAttribute('aria-label')}"]`
                            }))
                        };
                    });
                }, [], function (result) {
                    // Randomize candidate selection to simulate real-world
                    // voting patterns and avoid hardcoding
                    if (result.status === 0 && Array.isArray(result.value)) {
                        result.value.forEach(contest => {
                            console.log(`\nContest ${contest.contestIndex}: Max Votes = ${contest.maxVotes}`);
                            const shuffledCandidates = contest.candidates.sort(() => 0.5 - Math.random());
                            const selectedCandidates = shuffledCandidates.slice(0, contest.maxVotes);
                            selectedCandidates.forEach(candidate => {
                                console.log(`Voting for: ${candidate.name}`);
                                browser
                                    // Wait for each candidate checkbox to be
                                    // present before clicking
                                    .waitForElementPresent(candidate.selector, 20000)
                                    .click(candidate.selector)
                                    .pause(500)
                                    // Assert to catch UI issues early
                                    .assert.elementPresent(`${candidate.selector}:checked`, `Checkbox for ${candidate.name} is checked`)
                                    .pause(500);
                            });
                        });
                    } else {
                        browser.assert.fail('Failed to retrieve voting data');
                    }
                })
                .perform(() => maybeScreenshot(`screenshots/votingCompleted${iteration}.png`))
                // Proceed through the ballot and cast the vote
                .click("button.next-button")
                .pause(500)
                .waitForElementVisible("button.cast-ballot-button", 20000)
                .click("button.cast-ballot-button")
                .pause(500)
                .waitForElementVisible("button.ok-button", 20000)
                .click("button.ok-button")
                .pause(2500)
                .perform(() => maybeScreenshot(`screenshots/summaryPage${iteration}.png`))
                // Always log out to ensure the next iteration starts fresh
                .click("button.logout-button")
                .pause(500)
                .waitForElementVisible("li.logout-button", 20000)
                .click('li.logout-button')
                .pause(500)
                .click("button.ok-button")
                .pause(500)
                // Recursively call vote() for continuous load
                .perform(() => {
                    console.log(`Completed iteration #${iteration}`);
                    vote(iteration + 1);
                });
        }
        
        vote(1);
    }
};
