module.exports = {
    'Automated Voting Test (Robust)': function (browser) {
        const url = process.env.LOADTESTING_VOTING_URL;
        const password = "User1234567!";
        const otpCode = "123456";
        const numberOfVotes = 100;

        function vote(iteration) {
            const randomNumber = Math.floor(Math.random() * numberOfVotes) + 1;
            let rndStr = randomNumber > 9? `${randomNumber}` : `0${randomNumber}`;
            
            const username = "user01";
            console.log(`Starting iteration ${iteration} for: ${username}`);

            browser
                .url(url)
                .waitForElementVisible('body', 20000)
                .saveScreenshot(`screenshots/login(${iteration}).png`)
                .setValue('input[name=username]', username)
                .setValue('input[name=password]', username)
                .pause(500)
                .click('#kc-login')
                .pause(2500)
                .saveScreenshot(`screenshots/ballot_list_load${iteration}.png`)
                .waitForElementVisible("button.click-to-vote-button", 20000)
                .saveScreenshot(`screenshots/votingScreen${iteration}.png`)
                
                // More robust click for headless mode
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
                .saveScreenshot(`screenshots/afterClickToVote${iteration}.png`)
                .waitForElementVisible("button.start-voting-button", 20000)
                
                // More robust click for start voting button
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
                .pause(2000)
                .saveScreenshot(`screenshots/afterStartVoting${iteration}.png`)
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
                .saveScreenshot(`screenshots/votingCompleted${iteration}.png`)
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
                .saveScreenshot(`screenshots/afterCastBallot${iteration}.png`)
                
                // Look for various possible confirmation buttons
                .execute(function() {
                    const button = document.querySelector('button.finish-button');
                    if (button && !button.disabled) {
                        button.click();
                        return true;
                    }
                    return false;
                }, [], function(result) {
                    console.log('Finish button clicked via JavaScript:', result.value);
                })
                .saveScreenshot(`screenshots/summaryPage${iteration}.png`)
                .pause(500)
                // Continue with logout flow or handle completion
                .perform(() => {
                    console.log(`Completed iteration #${iteration}`);
                    // For now, just end the test instead of recursing
                    browser.end();
                });
        }

        vote(1);
    }
};
