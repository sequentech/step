module.exports = {
    'Automated Voting Test': function (browser) {
        const url = process.env.LOADTESTING_VOTING_URL;
        const password = "User1234567!";
        const otpCode = "123456";
        const numberOfVotes = 4032;

        function vote(iteration) {
            const randomNumber = Math.floor(Math.random() * numberOfVotes) + 1;
            const username = `testsequent2025+${randomNumber}@mailinator.com`;
            console.log(`Starting iteration ${iteration} for: ${username}`);

            browser
                .url(url)
                .waitForElementVisible('body', 20000)
                .saveScreenshot(`screenshots/login(${iteration}).png`)
                .setValue('input[name=username]', username)
                .setValue('input[name=password]', password)
                .pause(500)
                .click('#kc-login')
                .pause(2500)
                .saveScreenshot(`screenshots/ballot_list_load${iteration}.png`)
                .waitForElementVisible("button.click-to-vote-button", 20000)
                .saveScreenshot(`screenshots/votingScreen${iteration}.png`)
                .click("button.click-to-vote-button")
                .pause(500)
                .waitForElementVisible("button.start-voting-button", 20000)
                .click("button.start-voting-button")
                .pause(1000)
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
                                    .click(candidate.selector)
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
                .click("button.next-button")
                .pause(500)
                .waitForElementVisible("button.cast-ballot-button", 20000)
                .click("button.cast-ballot-button")
                .pause(500)
                .waitForElementVisible("button.ok-button", 20000)
                .click("button.ok-button")
                .pause(2500)
                .saveScreenshot(`screenshots/summaryPage${iteration}.png`)
                .click("button.logout-button")
                .pause(500)
                .waitForElementVisible("li.logout-button", 20000)
                .click('li.logout-button')
                .pause(500)
                .click("button.ok-button")
                .pause(500)
                .perform(() => {
                    console.log(`Completed iteration #${iteration}`);
                    vote(iteration + 1);
                });
        }

        vote(1);
    }
};
