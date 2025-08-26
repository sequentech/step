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
 *   export VOTING_URL='https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login'
 * 
 *   # Optional environment variables:
 *   export NUMBER_OF_VOTERS='4096'
 *   export NUMBER_OF_ITERATIONS='10'
 *   export PASSWORD_PATTERN='user{n}'
 *   export USERNAME_PATTERN='user{n}'
 *   export SAVE_SCREENSHOTS='false'
 * 
 *   # Run the script with Nightwatch:
 *   npx nightwatch voting.js --env chrome
 *
 * Environment variables:
 *   VOTING_URL - (example: https://voting-test.sequent.vote/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/7d7f840a-4e75-4ba4-b431-633196da1a2c/login) Voting URL
 *   PASSWORD_PATTERN     - (default: 'user{n}') Password pattern for all users,
 *                          n, {n} will be replaced with the user number
 *   NUMBER_OF_VOTERS     - (default: 4096) Number of unique users to simulate
 *   NUMBER_OF_ITERATIONS - (default: NUMBER_OF_VOTERS) Number of votes to be cast
 *   USERNAME_PATTERN     - (default: 'user{n}') 
 *                          Username pattern, {n} will be replaced with a random
 *                          number
 *   SAVE_SCREENSHOTS     - (default: 'false') Whether to save screenshots
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
        let url = process.env.VOTING_URL;
        const passwordPattern = process.env.PASSWORD_PATTERN || "user{n}";
        const numberOfVoters = parseInt(process.env.NUMBER_OF_VOTERS, 10) || 4096;
        const usernamePattern = process.env.USERNAME_PATTERN || 'user{n}';
        const saveScreenshots = (process.env.SAVE_SCREENSHOTS || 'false').toLowerCase() !== 'false';
        const numberOfIterations = process.env.NUMBER_OF_ITERATIONS && parseInt(process.env.NUMBER_OF_ITERATIONS, 10) || numberOfVoters;

        function getRandomIndex(numberOfVoters) {
            // Using a random number ensures we hit a wide range of test users
            // and avoid hardcoding patterns
            const randomNumber = Math.floor(Math.random() * numberOfVoters) + 1;
            const rndStr = `${randomNumber}`;
            return rndStr;
        }

        // Username pattern is parameterized
        function getPattern(rndStr, pattern) {
            return pattern.replace('{n}', rndStr);
        }

        // Helper to optionally save screenshots
        function maybeScreenshot(path) {
            if (saveScreenshots) {
                browser.saveScreenshot(path);
            }
            return browser;
        }

        // Robustly set value into an input selector and verify it.
        // Strategy:
        // 1) wait for present/visible; scroll + focus
        // 2) clearValue + setValue
        // 3) verify; if mismatch, JS-set value and dispatch input/change
        function robustSetValue(selector, value) {
        browser
            .waitForElementPresent(selector, 20000)
            .waitForElementVisible(selector, 20000)
            .execute(function (sel) {
            const el = document.querySelector(sel);
            if (!el) return false;
            try { el.scrollIntoView({ block: 'center', inline: 'nearest' }); } catch (e) {}
            try { el.focus(); } catch (e) {}
            return true;
            }, [selector])
            .pause(50)
            .click(selector)
            .clearValue(selector)
            .setValue(selector, value)
            .pause(150)
            .execute(function (sel) {
            const el = document.querySelector(sel);
            return el ? el.value : null;
            }, [selector], function (res) {
            if (res.value !== value) {
                // Fallback: force set via JS + dispatch events so frameworks pick it up
                browser
                .execute(function (sel, val) {
                    const el = document.querySelector(sel);
                    if (!el) return 'missing';
                    try { el.focus(); } catch (e) {}
                    try {
                    // Use native value setter to avoid readonly issues on prototypes
                    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                    setter.call(el, '');
                    el.dispatchEvent(new Event('input', { bubbles: true }));
                    el.dispatchEvent(new Event('change', { bubbles: true }));
                    setter.call(el, val);
                    el.dispatchEvent(new Event('input', { bubbles: true }));
                    el.dispatchEvent(new Event('change', { bubbles: true }));
                    } catch (e) {
                    el.value = val;
                    el.dispatchEvent(new Event('input', { bubbles: true }));
                    el.dispatchEvent(new Event('change', { bubbles: true }));
                    }
                    return el.value;
                }, [selector, value])
                .pause(100);
            }
            })
            .assert.value(selector, value);
        }

        // Recursively call vote() to simulate continuous load; this is
        // intentional to generate a high load on the voting system
        function vote(iteration) {
            const idx = getRandomIndex(numberOfVoters)
            const username = getPattern(idx, usernamePattern);
            const password = getPattern(idx, passwordPattern);
            console.log(`Starting iteration ${iteration} for: ${username}`);
            
            browser
                // Always start from the login page to ensure a clean session
                .url(url)
                .waitForElementVisible('body', 20000)
                // Screenshots are optional for performance or disk space
                // reasons
                .perform(() => maybeScreenshot(`screenshots/login(${iteration}).png`))
                .pause(1500)
                // Use unique credentials per iteration to ensure enough total
                // votes are captured; please ensure revoting is allowed because
                // this script will sometimes vote multiple times with the same
                // user
                .perform(() => robustSetValue('input[name=username]', username))
                .perform(() => robustSetValue('input[name=password]', password))
                .pause(1500)
                .perform(() => maybeScreenshot(`screenshots/login_filled(${iteration}).png`))
                .execute(function() {
                    const button = document.querySelector('#kc-login');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('#kc-login button clicked via JavaScript:', result.value);
                })
                .pause(2500)
                .perform(() => maybeScreenshot(`screenshots/ballot_list_load${iteration}.png`))
                // Wait for the voting UI to load; this ensures the test doesn't
                // race ahead
                .waitForElementVisible("button.click-to-vote-button", 20000)
                .perform(() => maybeScreenshot(`screenshots/votingScreen${iteration}.png`))
                .execute(function() {
                    const button = document.querySelector('button.click-to-vote-button');
                    if (button) {
                        button.scrollIntoView();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    if (!result.value) {
                        console.log('Warning: click-to-vote-button not found');
                    }
                })
                .pause(1000)
                .perform(() => maybeScreenshot(`screenshots/startVotingScreen${iteration}.png`))
                .execute(function() {
                    const button = document.querySelector('button.click-to-vote-button');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Click to vote button clicked via JavaScript:', result.value);
                })
                .pause(2000)
                .waitForElementVisible("button.start-voting-button", 20000)
                .execute(function() {
                    const button = document.querySelector('button.start-voting-button');
                    if (button) {
                        button.scrollIntoView();
                        return true;
                    }
                    return false;
                })
                .pause(1000)
                .execute(function() {
                    const button = document.querySelector('button.start-voting-button');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Start voting button clicked via JavaScript:', result.value);
                })
                .pause(1000)
                .saveScreenshot(`screenshots/afterStartVoting${iteration}.png`)
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
                                    .waitForElementPresent(candidate.selector, 20000)
                                    // Use JavaScript click for better reliability in headless
                                    .execute(function(selector) {
                                        const element = document.querySelector(selector);
                                        if (element) {
                                            element.click();
                                            return true;
                                        }
                                        return false;
                                    }, [candidate.selector], function(clickResult) {
                                        console.log(`Clicked ${candidate.name} via JavaScript:`, clickResult.value);
                                    })
                                    .pause(500)
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
                .execute(function() {
                    const button = document.querySelector('button.next-button');
                    if (button) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Next button clicked via JavaScript:', result.value);
                })
                .pause(2000)
                .perform(() => maybeScreenshot(`screenshots/reviewScreen${iteration}.png`))
                .waitForElementVisible("button.cast-ballot-button", 20000)
                .execute(function() {
                    const button = document.querySelector('button.cast-ballot-button');
                    if (button) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Cast ballot button clicked via JavaScript:', result.value);
                })
                .pause(2000)
                .perform(() => maybeScreenshot(`screenshots/afterCastBallot${iteration}.png`))
                .waitForElementVisible("div.confirmation-screen.screen", 1500)
                .execute(function() {
                    const button = document.querySelector('button.logout-button');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Logout button clicked via JavaScript:', result.value);
                })
                .pause(500)
                .waitForElementVisible("li.logout-button", 20000)
                .execute(function() {
                    const button = document.querySelector('li.logout-button');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Logout button clicked via JavaScript:', result.value);
                })
                .pause(500)
                .execute(function() {
                    const button = document.querySelector('button.ok-button');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Final logout button clicked via JavaScript:', result.value);
                })
                .pause(500)
                .saveScreenshot(`screenshots/summaryPage${iteration}.png`)
                .pause(500)
                // Continue with logout flow or handle completion
                .perform(() => {
                    console.log(`Completed iteration #${iteration}`);

                    if (iteration >= numberOfIterations) {
                        browser.end();
                    } else {
                        vote(iteration + 1);
                    }
                });
        }
        
        vote(1);
    }
};