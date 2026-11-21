# TODO: Complete Tally Integration Test Setup

This document tracks the remaining work needed to make the tally integration test functional.

## Status: 🚧 Implementation In Progress

The GitHub Actions workflow, validation scripts, and documentation are complete, but the test fixtures need to be generated with actual election data.

## Remaining Tasks

### Critical (Required for test to run)

- [ ] **Generate actual election fixture JSON** (`election-fixture.json`)
  - Current status: Template file exists (`election-fixture.json.template`)
  - Required: Full election event export with encrypted votes
  - Steps:
    1. Create a test election event using step-cli or admin portal
    2. Configure: 1 election, 1 contest (Plurality at Large, 2 winners), 3 candidates, 1 area
    3. Create 10-20 test voters
    4. Cast test votes (e.g., Alice: 8, Bob: 6, Carol: 4)
    5. Complete key ceremony
    6. Export with: `cli step export-election-event --election-event-id <ID> --bulletin-board --tally --output-dir ./fixtures`
    7. Copy exported JSON to `election-fixture.json`
    8. Anonymize sensitive voter data (names, emails, etc.)
  - Owner: TBD
  - Effort: ~2-3 hours

- [ ] **Update expected results JSON** with actual values
  - Current status: Template with placeholder values
  - Required: Actual vote counts matching the fixture
  - Depends on: Election fixture generation
  - Steps:
    1. Record the IDs from the generated election fixture
    2. Update `expected-results.json` with:
       - Correct election_event_id
       - Correct election_id
       - Correct contest_id
       - Actual vote counts for each candidate
       - Correct winner determination
  - Owner: TBD
  - Effort: ~30 minutes

### Important (Improves test reliability)

- [ ] **Test workflow manually before merging**
  - Run locally using the instructions in `docs/testing/integration-tests.md`
  - Verify all steps execute successfully
  - Check that validation passes
  - Measure baseline performance
  - Owner: TBD
  - Effort: ~1 hour

- [ ] **Handle trustee key confirmation in workflow**
  - Current status: Commented out in workflow
  - Required: Automate trustee key confirmation or configure fixture with pre-confirmed keys
  - Options:
    - Option A: Include trustee keys in fixture (if possible)
    - Option B: Automate trustee authentication in workflow
    - Option C: Use test trustee credentials
  - Owner: TBD
  - Effort: ~2 hours (depends on option chosen)

- [ ] **Add status polling for tally completion**
  - Current status: Simple 5-minute timeout with placeholder check
  - Required: Actually query tally status from API or database
  - Implementation ideas:
    - Query Hasura for tally_session_execution status
    - Check Windmill task status via API
    - Poll results.db file existence
  - Owner: TBD
  - Effort: ~1 hour

- [ ] **Extract election event ID and tally ID from command output**
  - Current status: Hardcoded `<to-be-extracted>` placeholders
  - Required: Parse IDs from step-cli output
  - Steps:
    1. Capture stdout from import-election command
    2. Parse JSON or grep for UUID pattern
    3. Set environment variable
    4. Repeat for tally ID
  - Owner: TBD
  - Effort: ~30 minutes

### Nice to Have (Enhancements)

- [ ] **Add multiple test scenarios**
  - Single contest (current)
  - Multi-contest election
  - Instant Runoff Voting (IRV)
  - Different vote distributions
  - Edge cases (ties, empty ballots)
  - Owner: TBD
  - Effort: ~4 hours per scenario

- [ ] **Improve validation script**
  - Map actual database schema to validation queries
  - Add more detailed vote count checks
  - Validate report content (not just existence)
  - Check XLSX file content
  - Owner: TBD
  - Effort: ~2 hours

- [ ] **Add performance regression alerts**
  - Current: Basic >10% threshold warning
  - Enhancement: Track trends, alert on repeated slowdowns
  - Integration with monitoring tools
  - Owner: TBD
  - Effort: ~1-2 hours

- [ ] **Create fixture generation tool**
  - Automate the fixture creation process
  - Script that creates election, votes, and exports
  - Makes it easier to regenerate fixtures
  - Owner: TBD
  - Effort: ~3-4 hours

## Workflow Status

| Component | Status | Notes |
|-----------|--------|-------|
| GitHub Actions workflow | ✅ Complete | `.github/workflows/tally-integration-test.yml` |
| Validation script | ✅ Complete | `.github/scripts/validate-tally-results.py` |
| Documentation | ✅ Complete | `docs/testing/integration-tests.md` |
| README badge | ✅ Complete | Added to main README.md |
| Election fixture | ❌ Todo | Template exists, needs actual data |
| Expected results | ❌ Todo | Template exists, needs actual values |
| Trustee handling | ⚠️ Partial | Commented out in workflow |
| Status polling | ⚠️ Partial | Placeholder implementation |
| ID extraction | ⚠️ Partial | Hardcoded placeholders |

## Getting Started

If you're assigned to complete this work:

1. **Read the documentation:**
   - [Integration Tests Documentation](../../../docs/testing/integration-tests.md)
   - [Test Fixtures README](./README.md)
   - [Step CLI README](../../README.md)

2. **Set up local environment:**
   - Follow the "Running Locally" section in the documentation
   - Verify you can run step-cli commands successfully

3. **Generate the fixture:**
   - Follow the steps in the "Critical" section above
   - Test import locally before committing

4. **Test the workflow:**
   - Push to a feature branch
   - Monitor the GitHub Actions run
   - Fix any issues that arise

5. **Update this TODO:**
   - Check off completed items
   - Add any new issues discovered
   - Update effort estimates if needed

## Questions?

- Check the [troubleshooting section](../../../docs/testing/integration-tests.md#troubleshooting) in the documentation
- Review the workflow file for implementation details
- Look at the validation script to understand what's being checked

## Timeline Estimate

Minimum time to completion: ~4-5 hours (critical tasks only)
Full completion: ~10-15 hours (including all enhancements)

This estimate assumes familiarity with step-cli and the election system.
