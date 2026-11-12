# Tally Integration Test Fixtures

This directory contains test fixtures for the GitHub Actions tally integration test workflow.

## Available Fixtures

### 1. `election-fixture.json` (PRIMARY)
**Complete election fixture with full i18n support**

- **Election Event**: Integration Test Election
- **Elections**: 1 (Test Election 2025)
- **Contests**: 1 (Plurality at Large, select top 2)
- **Candidates**: 3 (Alice, Bob, Carol)
- **Areas**: 1 (Test District)
- **Features**:
  - Full i18n translations (8 languages: en, es, cat, fr, tl, gl, nl, eu)
  - Complete presentation objects for all entities
  - Proper status initialization
  - Voting channels configuration
  - Ready for import via step-cli

**Use this fixture for**: Standard tally integration tests in CI/CD

---

### 2. `election-fixture-minimal.json`
**Minimal election fixture - bare essentials**

- **Election Event**: Minimal Test
- **Elections**: 1 (Election 1)
- **Contests**: 1 (Question 1 - Yes/No)
- **Candidates**: 2 (Yes, No)
- **Areas**: 1 (District A)
- **Features**:
  - Minimal i18n (English only)
  - Simplest possible valid structure
  - Ideal for testing import/export basics

**Use this fixture for**: Quick smoke tests, import validation, baseline testing

---

### 3. `election-fixture-irv.json`
**IRV/Ranked Choice Voting election fixture**

- **Election Event**: IRV Test Election
- **Elections**: 1 (Mayor 2025)
- **Contests**: 1 (IRV/RCV contest - rank up to 4 candidates)
- **Candidates**: 4 (Alice Johnson, Bob Smith, Carol Green, David Brown)
- **Areas**: 1 (RCV District)
- **Features**:
  - Instant Runoff Voting algorithm
  - Ranked choice ballot configuration
  - Tally configuration for IRV method
  - min_votes: 1, max_votes: 4 (ranking positions)

**Use this fixture for**: IRV/RCV tally testing, ranked choice validation

---

## Expected Results

### `expected-results.json`
Expected tally results for the primary `election-fixture.json`:

- **Alice**: 8 votes (Winner, Rank 1)
- **Bob**: 6 votes (Winner, Rank 2)
- **Carol**: 4 votes (Not winner, Rank 3)
- **Total votes**: 18
- **Total ballots**: 10
- **Winners**: 2 (as configured in contest)

The validation script checks:
- Vote counts match expected values
- Winners are correctly identified
- Database structure is complete
- Required files are generated (results.db, *.html, etc.)

---

## Usage in CI/CD

### GitHub Actions Workflow
The `.github/workflows/tally-integration-test.yml` workflow:

1. **Import** the fixture using `step import-election`
2. **Configure** trustees and key ceremony
3. **Start tally** ceremony
4. **Complete tally** and download results
5. **Validate** against `expected-results.json`

### Local Testing
```bash
# Import the fixture
seq step import-election \
  --file-path ./test-fixtures/tally-integration-test/election-fixture.json

# Run tally (requires proper setup)
seq step start-tally \
  --election-event-id test-event-001 \
  --tally-type ELECTORAL_RESULTS

# Download and validate results
seq step download-tally-results \
  --election-event-id test-event-001 \
  --tally-id <TALLY_ID> \
  --output-dir ./tally-output
```

---

## Key Differences from CLI-Generated Exports

These fixtures address the issues identified in the CLI vs Admin Portal export comparison:

✅ **All fixtures include**:
- `election_event.presentation` with full configuration
- `elections[].presentation` with i18n and language_conf
- `elections[].status` with voting status tracking
- `elections[].voting_channels` configuration
- `contests[].presentation` with candidates_order and i18n
- `candidates[].presentation` with i18n translations
- `candidates[].is_public` set to `false` (not null)
- `areas[].presentation` with early voting configuration

These fixtures can be used as **reference implementations** for fixing the CLI export/import issues documented in the bug report.

---

## Creating New Fixtures

To create custom test fixtures:

### Option 1: From Admin Portal Export
1. Create an election using the Admin Portal
2. Configure as needed (contests, candidates, areas)
3. Export using the Admin Portal export feature
4. Simplify/anonymize the exported JSON
5. Update IDs to be test-friendly (e.g., `test-event-001`)

### Option 2: From Existing Fixture
1. Copy one of the existing fixtures
2. Modify IDs, names, and structure as needed
3. Update the corresponding expected-results file
4. Test import: `seq step import-election --file-path your-fixture.json`

### Option 3: From CLI (Requires Fixes)
1. Create using step-cli commands
2. Export with: `seq step export-election-event --election-event-id <ID>`
3. **Important**: Currently CLI exports are missing presentation objects
4. Manually add missing fields using these fixtures as templates

---

## Fixture Structure Reference

### Required Fields for Import

**Election Event**:
- `id`, `name`, `tenant_id`, `encryption_protocol`
- `presentation` (with i18n, language_conf, policies)
- `voting_channels`
- `bulletin_board_reference`

**Elections**:
- `id`, `name`, `election_event_id`
- `presentation` (with i18n, language_conf)
- `status` (with voting statuses and period dates)
- `voting_channels`

**Contests**:
- `id`, `name`, `election_id`, `election_event_id`
- `counting_algorithm`, `max_votes`, `min_votes`, `winning_candidates_num`
- `is_active`, `is_encrypted`
- `presentation` (with candidates_order, i18n)

**Candidates**:
- `id`, `name`, `contest_id`, `election_event_id`
- `is_public` (boolean, not null)
- `presentation` (with i18n)

**Areas**:
- `id`, `name`, `election_event_id`
- `presentation` (with allow_early_voting)

**Area Contests**:
- `id`, `area_id`, `contest_id`

---

## Validation Scripts

### Python Validation Script
Located at: `.github/scripts/validate-tally-results.py`

Validates:
- Vote counts match expected values
- Winners are correctly identified
- Database tables exist and have correct row counts
- Required files are present

### SQLite Database Checks
The tally results database (`results.db`) should contain:
- `election_event`, `election`, `contest`, `area`, `candidate` tables
- `results_contest`, `results_contest_candidate` tables
- Proper foreign key relationships

---

## Notes

### Bulletin Board Data
**Important**: These fixtures do NOT include actual bulletin board data (encrypted votes). For full end-to-end testing:
1. Import the fixture
2. Cast test votes through the voting portal
3. Complete key ceremony
4. Run tally

For CI/CD, you may need to:
- Pre-generate bulletin board data
- Mock the voting/encryption process
- Or use a simplified tally path for testing

### Keycloak Configuration
The `keycloak_event_realm` section is intentionally minimal. For real deployments:
- Full Keycloak realm configuration would be required
- Authentication flows, clients, and user federation
- These fixtures focus on data structure, not auth configuration

---

## Troubleshooting

### Import Fails
- Check that all required fields are present
- Verify IDs are unique and properly referenced
- Ensure tenant_id matches your environment
- Check presentation objects are not null

### Tally Fails
- Verify bulletin board data exists
- Check key ceremony is complete
- Ensure voting status allows tally
- Validate contest configuration (max_votes, algorithm)

### Validation Fails
- Check expected-results.json matches fixture IDs
- Verify vote counts in expected results
- Ensure database checks match your schema version

---

## Related Files

- [Bug Report: CLI Export Missing Properties](/home/vscode/.claude/plans/squishy-juggling-lemur.md)
- [GitHub Workflow: Tally Integration Test](/.github/workflows/tally-integration-test.yml)
- [Validation Script](/.github/scripts/validate-tally-results.py)

---

## Changelog

### 2025-02-10
- ✅ Created primary `election-fixture.json` with full i18n
- ✅ Created `election-fixture-minimal.json` for smoke tests
- ✅ Created `election-fixture-irv.json` for RCV testing
- ✅ Updated `expected-results.json` with proper validation rules
- ✅ All fixtures include proper presentation objects (fixes CLI export bug)
