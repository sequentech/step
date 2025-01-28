client => {
    const url = "{url}"
    const password = "Aa1234567!"
    const otpCode = "123456"
    const numberOfVotes = "{numberOfVoters}"
    
    const randomNumber = Math.floor(Math.random() * parseInt(numberOfVotes)) + 1
    const username = `user${randomNumber}@gmail.com`

    client.url(url)
    .waitForElementVisible('body', 1000)
    .setValue('input[name=username]', username)
    .setValue('input[name=password]', password)
    .pause(500)
    .click('#kc-login')
    .pause(2000)
    // client.waitForElementVisible('otp-container', 2000)
    // for (let otpIndex = 1; otpIndex <= otpLength; otpIndex++) {
    //     client.setValue(`*[id='otp-${otpIndex}']`, otpCode.at(otpIndex - 1));
    // }
    // client.pause(500)
    // .click(submitButton)
    console.log(client.candidate)
    console.log(client.client)

    client.saveScreenshot('screenshots/votingScreen.png')
    .pause(3000)
    .click("button.click-to-vote-button")
    .pause(500)
    .waitForElementVisible("button.start-voting-button", 10000)
    .click("button.start-voting-button")
    .pause(1000)

    .execute(function() {

        const contests = document.querySelectorAll('div[class^="contest-"]');
        const votingData = [];

        contests.forEach((contest, contestIndex) => {

            const titleElement = contest.querySelector('h5[data-max]');
            const maxVotes = parseInt(titleElement.getAttribute('data-max'), 10);

            const candidateCheckboxes = contest.querySelectorAll('input[type="checkbox"][aria-label]');

            votingData.push({
                contestIndex: contestIndex,
                maxVotes: maxVotes,
                candidates: Array.from(candidateCheckboxes).map(checkbox => ({
                    name: checkbox.getAttribute('aria-label'),
                    selector: `input[aria-label="${checkbox.getAttribute('aria-label')}"]`
                }))
            });
        });

        return votingData;
    }, [], function(result) {
         if (result.status === 0 && Array.isArray(result.value)) {

                result.value.forEach(contest => {
                    console.log(`\nContest ${contest.contestIndex}: Max Votes = ${contest.maxVotes}`);

                    const shuffledCandidates = contest.candidates.sort(() => 0.5 - Math.random());

                    const selectedCandidates = shuffledCandidates.slice(0, contest.maxVotes);

                    selectedCandidates.forEach((candidate, voteIndex) => {
                        console.log(`Voting for: ${candidate.name} (${voteIndex + 1}/${contest.maxVotes})`);
                        console.log(candidate.selector);

                        client
                            .waitForElementPresent(candidate.selector, 10000) 
                            .click(candidate.selector)
                            .pause(500) 
                            .assert.elementPresent(`${candidate.selector}:checked`, `Checkbox for ${candidate.name} is checked`)
                            .pause(500);
                    });
                });
            } else {
                client.assert.fail('Failed to retrieve voting data');
            }
        })

    .saveScreenshot('screenshots/votingCompleted.png')
    .click("button.next-button")
    .pause(500)
    .waitForElementVisible("button.cast-ballot-button", 20e3)
    .click("button.cast-ballot-button")
    .pause(500)
    .waitForElementVisible("button.ok-button", 20e3)
    .click("button.ok-button")
    .pause(500)
    .saveScreenshot('screenshots/summaryPage.png')
    .end();

}
