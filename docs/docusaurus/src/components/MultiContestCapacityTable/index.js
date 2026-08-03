import React from 'react';

// sequent_core::ballot_codec::multi_ballot::BallotChoices::MAX_SIZE_BYTES
const MAX_SIZE_BYTES = 29;

const CANDIDATES_PER_CONTEST = [2, 5, 10, 20, 50, 100];

// Per-contest factor for a single-choice (max_votes = 1) plurality-at-large
// contest with no decline-to-vote flag and no explicit blank marker: base 2
// for the explicit invalid flag times base (candidates + 1) for the one
// ordinary-candidate slot (Section 11.4).
function contestsNeeded(candidatesPerContest) {
  const limit = 256n ** BigInt(MAX_SIZE_BYTES);
  const factor = 2n * BigInt(candidatesPerContest + 1);

  let product = 1n;
  let count = 0;
  while (product <= limit) {
    product *= factor;
    count += 1;
  }
  return count;
}

export default function MultiContestCapacityTable() {
  return (
    <table>
      <thead>
        <tr>
          <th>Candidates per contest</th>
          <th style={{textAlign: 'right'}}>Contests needed to exceed {MAX_SIZE_BYTES} bytes</th>
        </tr>
      </thead>
      <tbody>
        {CANDIDATES_PER_CONTEST.map((candidates) => (
          <tr key={candidates}>
            <td>{candidates}</td>
            <td style={{textAlign: 'right'}}>{contestsNeeded(candidates)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
