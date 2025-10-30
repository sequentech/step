// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Automated Voting Test Script for Nightwatch.js
 *
 * See the header comment in the original file for usage & env-vars.
 */

const fs = require('fs');
const path = require('path');

const PARALLEL_DIR = process.env.PARALLEL_DIR || '.';
const ENABLE_VOTER_TRACKING = (process.env.ENABLE_VOTER_TRACKING || 'true').toLowerCase() === 'true';
const USED_VOTERS_FILE = path.join(PARALLEL_DIR, 'used_voters.txt');
const USED_VOTERS_LOCK = path.join(PARALLEL_DIR, 'used_voters.lock');
const MAX_VOTER_CLAIM_ATTEMPTS = parseInt(process.env.VOTER_CLAIM_MAX_RETRIES || '100', 10);

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------
try { fs.mkdirSync(PARALLEL_DIR, { recursive: true }); } catch (_) {}

function readUsedVotersSet() {
  try {
    const data = fs.readFileSync(USED_VOTERS_FILE, 'utf8');
    return new Set(data.split('\n').map(s => s.trim()).filter(Boolean));
  } catch (err) {
    if (err.code === 'ENOENT') return new Set();
    throw err;
  }
}

function appendUsedVoter(username) {
  fs.appendFileSync(USED_VOTERS_FILE, username + '\n', { encoding: 'utf8', flag: 'a' });
}

function releaseLock(lockPath) {
  try { fs.unlinkSync(lockPath); } catch (_) {}
}

// ---------------------------------------------------------------------------
// Synchronous voter claiming (used when ENABLE_VOTER_TRACKING === true)
// ---------------------------------------------------------------------------
function claimRandomVoterSync(getRandomCandidate) {
  for (let attempt = 1; attempt <= MAX_VOTER_CLAIM_ATTEMPTS; attempt++) {
    const candidate = getRandomCandidate();

    // --- acquire lock -------------------------------------------------------
    let lockAcquired = false;
    for (let i = 0; i < 10 && !lockAcquired; i++) {
      try {
        fs.writeFileSync(USED_VOTERS_LOCK, 'lock', { flag: 'wx' });
        lockAcquired = true;
      } catch (err) {
        if (err.code !== 'EEXIST') throw err;
        // tiny sleep before next try
        require('deasync').sleep(5);
      }
    }
    if (!lockAcquired) continue;

    try {
      const used = readUsedVotersSet();
      if (!used.has(candidate)) {
        appendUsedVoter(candidate);
        if (attempt > 1) {
          console.warn(`[voting] Claimed voter after ${attempt} attempts: ${candidate}`);
        }
        return candidate;
      }
    } finally {
      releaseLock(USED_VOTERS_LOCK);
    }

    // collision -> retry
    if (attempt % 5 === 0) {
      console.warn(`[voting] Collision for ${candidate} (attempt ${attempt}/${MAX_VOTER_CLAIM_ATTEMPTS})`);
    }
  }
  throw new Error(`Unable to claim a unique voter after ${MAX_VOTER_CLAIM_ATTEMPTS} attempts`);
}

// ---------------------------------------------------------------------------
// Nightwatch test
// ---------------------------------------------------------------------------
module.exports = {
  'Automated Voting Test': function (browser) {
    // ---- configuration ----------------------------------------------------
    const url = process.env.VOTING_URL;
    const passwordPattern = process.env.PASSWORD_PATTERN || 'user{n}';
    const numberOfVoters = parseInt(process.env.NUMBER_OF_VOTERS, 10) || 4096;
    const voterMinIndex = parseInt(process.env.VOTER_MIN_INDEX, 10) || 1;
    const usernamePattern = process.env.USERNAME_PATTERN || 'user{n}';
    const candidatesPatternStr = process.env.CANDIDATES_PATTERN || '';
    let candidatesPattern = null;
    if (candidatesPatternStr) {
      try {
        // Remove leading and trailing slashes if present, extract pattern and flags
        const match = candidatesPatternStr.match(/^\/(.*)\/([gimuy]*)$/);
        if (match) {
          candidatesPattern = new RegExp(match[1], match[2]);
        } else {
          // If not in /pattern/flags format, use as-is
          candidatesPattern = new RegExp(candidatesPatternStr);
        }
      } catch (e) {
        console.error('Invalid regex pattern:', candidatesPatternStr, e.message);
      }
    }
    const saveScreenshots = (process.env.SAVE_SCREENSHOTS || 'false').toLowerCase() !== 'false';
    const numberOfIterations = process.env.NUMBER_OF_ITERATIONS
      ? parseInt(process.env.NUMBER_OF_ITERATIONS, 10)
      : numberOfVoters;

    // ---- helpers ----------------------------------------------------------
    const getRandomIndex = () => {
      const rnd = Math.floor(Math.random() * numberOfVoters) + voterMinIndex;
      return `${rnd}`;
    };

    const getPattern = (rndStr, pattern) => pattern.replace('{n}', rndStr);

    const maybeScreenshot = (file) => {
      if (saveScreenshots) browser.saveScreenshot(file);
    };

    // robust input fill (fallback to JS when Nightwatch fails)
    const robustSetValue = (selector, value) => {
      browser
        .waitForElementPresent(selector, 20000)
        .waitForElementVisible(selector, 20000)
        .execute(
          sel => {
            const el = document.querySelector(sel);
            if (el) {
              el.scrollIntoView({ block: 'center' });
              el.focus();
            }
            return !!el;
          },
          [selector]
        )
        .pause(50)
        .click(selector)
        .clearValue(selector)
        .setValue(selector, value)
        .pause(150)
        .execute(
          sel => document.querySelector(sel)?.value,
          [selector],
          res => {
            if (res.value !== value) {
              browser.execute(
                (sel, val) => {
                  const el = document.querySelector(sel);
                  if (!el) return;
                  const setter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype,
                    'value'
                  ).set;
                  setter.call(el, val);
                  el.dispatchEvent(new Event('input', { bubbles: true }));
                  el.dispatchEvent(new Event('change', { bubbles: true }));
                },
                [selector, value]
              );
            }
          }
        )
        .assert.value(selector, value);
    };

    // ---- recursive voting loop --------------------------------------------
    const vote = (iteration) => {
      // claim unique voter (or just random)
      const idx = ENABLE_VOTER_TRACKING
        ? claimRandomVoterSync(getRandomIndex)
        : getRandomIndex();
      const username = getPattern(idx, usernamePattern);
      const password = getPattern(idx, passwordPattern);
      console.log(`[iteration ${iteration}] ${username}`);

      browser
        .url(url)
        .waitForElementVisible('body', 20000)
        .perform(() => maybeScreenshot(`screenshots/login(${iteration}).png`))
        .pause(1500)

        // ---- login --------------------------------------------------------
        .perform(() => robustSetValue('input[name=username]', username))
        .perform(() => robustSetValue('input[name=password]', password))
        .pause(1500)
        .perform(() => maybeScreenshot(`screenshots/login_filled(${iteration}).png`))
        .execute(
          () => {
            const btn = document.querySelector('#kc-login');
            if (btn && !btn.disabled) btn.click();
            return !!btn;
          },
          [],
          r => console.log('#kc-login clicked:', r.value)
        )
        .pause(2500)
        .perform(() => maybeScreenshot(`screenshots/ballot_list_load${iteration}.png`))

        // ---- start voting -------------------------------------------------
        .waitForElementVisible('button.click-to-vote-button', 20000)
        .perform(() => maybeScreenshot(`screenshots/votingScreen${iteration}.png`))
        .execute(
          () => {
            const btn = document.querySelector('button.click-to-vote-button');
            if (btn) btn.scrollIntoView();
            return !!btn;
          },
          [],
          r => r.value || console.warn('click-to-vote-button missing')
        )
        .pause(1000)
        .execute(
          () => {
            const btn = document.querySelector('button.click-to-vote-button');
            if (btn && !btn.disabled) btn.click();
            return !!btn;
          },
          [],
          r => console.log('click-to-vote clicked:', r.value)
        )
        .pause(2000)
        .waitForElementVisible('button.start-voting-button', 20000)
        .execute(
          () => {
            const btn = document.querySelector('button.start-voting-button');
            if (btn && !btn.disabled) btn.click();
            return !!btn;
          },
          [],
          r => console.log('start-voting clicked:', r.value)
        )
        .pause(1000)
        .perform(() => maybeScreenshot(`screenshots/afterStartVoting${iteration}.png`))

        // ---- dynamic candidate selection ----------------------------------
        .execute(
          () => {
            const contests = document.querySelectorAll('div[class^="contest-"]');
            return Array.from(contests).map((c, i) => {
              const title = c.querySelector('h5[data-max]');
              const max = parseInt(title.getAttribute('data-max'), 10);
              const boxes = c.querySelectorAll('input[type="checkbox"][aria-label]');
              let candidates = Array.from(boxes).map(cb => ({
                  name: cb.getAttribute('aria-label'),
                  selector: `input[aria-label="${cb.getAttribute('aria-label')}"]`
                }));
              if (candidatesPattern) {
                candidates = candidates
                  .filter(candidate => candidatesPattern.test(candidate.name))
              }

              return {
                contestIndex: i,
                maxVotes: max,
                candidates,
              };
            });
          },
          [],
          result => {
            if (result.status !== 0 || !Array.isArray(result.value)) {
              return browser.assert.fail('Failed to read contests');
            }

            result.value.forEach(contest => {
              const shuffled = contest.candidates.sort(() => 0.5 - Math.random());
              const chosen = shuffled.slice(0, contest.maxVotes);

              chosen.forEach(cand => {
                console.log(`  -> ${cand.name}`);
                browser
                  .waitForElementPresent(cand.selector, 20000)
                  .execute(
                    sel => {
                      const el = document.querySelector(sel);
                      if (el) el.click();
                      return !!el;
                    },
                    [cand.selector],
                    r => console.log(`  clicked ${cand.name}:`, r.value)
                  )
                  .pause(500)
                  .assert.elementPresent(`${cand.selector}:checked`);
              });
            });
          }
        )
        .perform(() => maybeScreenshot(`screenshots/votingCompleted${iteration}.png`))

        // ---- review & cast ------------------------------------------------
        .execute(
          () => {
            const btn = document.querySelector('button.next-button');
            if (btn) btn.click();
            return !!btn;
          },
          [],
          r => console.log('next button clicked:', r.value)
        )
        .pause(2000)
        .perform(() => maybeScreenshot(`screenshots/reviewScreen${iteration}.png`))
        .waitForElementVisible('button.cast-ballot-button', 20000)
        .execute(
          () => {
            const btn = document.querySelector('button.cast-ballot-button');
            if (btn) btn.click();
            return !!btn;
          },
          [],
          r => console.log('cast ballot clicked:', r.value)
        )
        .pause(2000)
        .perform(() => maybeScreenshot(`screenshots/afterCastBallot${iteration}.png`))
        .waitForElementVisible('div.confirmation-screen.screen', 1500)

        // ---- logout -------------------------------------------------------
        .execute(
          () => {
            const btn = document.querySelector('button.logout-button');
            if (btn && !btn.disabled) btn.click();
            return !!btn;
          },
          [],
          r => console.log('logout button clicked:', r.value)
        )
        .pause(500)
        .waitForElementVisible('li.logout-button', 20000)
        .execute(
          () => {
            const btn = document.querySelector('li.logout-button');
            if (btn && !btn.disabled) btn.click();
            return !!btn;
          },
          [],
          r => console.log('li.logout-button clicked:', r.value)
        )
        .pause(500)
        .execute(
          () => {
            const btn = document.querySelector('button.ok-button');
            if (btn && !btn.disabled) btn.click();
            return !!btn;
          },
          [],
          r => console.log('ok-button clicked:', r.value)
        )
        .pause(500)
        .perform(() => maybeScreenshot(`screenshots/summaryPage${iteration}.png`))
        .pause(500)

        // ---- next iteration -----------------------------------------------
        .perform(() => {
          console.log(`Completed iteration #${iteration}`);
          if (iteration >= numberOfIterations) {
            browser.end();
          } else {
            vote(iteration + 1);
          }
        });
    };

    // start the loop
    vote(1);
  }
};