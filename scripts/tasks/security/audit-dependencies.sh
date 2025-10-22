#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Note: We don't use 'set -e' because we want to continue auditing even if some tools fail
# Instead, we track errors and report them at the end

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Counters
TOTAL_VULNERABILITIES=0
CRITICAL_COUNT=0
HIGH_COUNT=0
MODERATE_COUNT=0
LOW_COUNT=0
ERROR_COUNT=0

# Arrays to store failed audits and manual commands
declare -a FAILED_AUDITS
declare -a MANUAL_COMMANDS
declare -a ERRORS

echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Security Audit - Dependency Vulnerability Scanner${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""

# Function to print section header
print_section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to audit Yarn packages
audit_yarn() {
    local dir=$1
    local name=$2
    
    if [ ! -f "$dir/package.json" ] || [ ! -f "$dir/yarn.lock" ]; then
        return
    fi
    
    echo -e "${YELLOW}📦 Auditing (Yarn): $name${NC}"
    cd "$dir"
    
    if ! command_exists yarn; then
        echo -e "  ${RED}✗ ERROR: yarn not found${NC}"
        ERRORS+=("yarn not found - cannot audit $name")
        FAILED_AUDITS+=("$name (yarn)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && yarn audit")
        MANUAL_COMMANDS+=("# To fix (uses resolutions in package.json):")
        MANUAL_COMMANDS+=("cd $dir && yarn audit --json | grep -v '^{\"type\":\"info\"' # Review and add resolutions")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    AUDIT_OUTPUT=$(yarn audit --json 2>&1)
    AUDIT_EXIT_CODE=$?
    
    if [ $AUDIT_EXIT_CODE -ne 0 ] && ! echo "$AUDIT_OUTPUT" | grep -q "auditSummary"; then
        echo -e "  ${RED}✗ ERROR: yarn audit failed${NC}"
        ERRORS+=("yarn audit failed for $name")
        FAILED_AUDITS+=("$name (yarn)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && yarn audit")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    VULN_COUNT=$(echo "$AUDIT_OUTPUT" | jq -s 'map(select(.type == "auditSummary")) | .[0].data.vulnerabilities | to_entries | map(.value) | add // 0' 2>/dev/null || echo "0")
    
    if [ "$VULN_COUNT" -gt 0 ]; then
        echo -e "  ${RED}✗ Found $VULN_COUNT vulnerabilities${NC}"
        
        # Parse severity counts
        CRIT=$(echo "$AUDIT_OUTPUT" | jq -s 'map(select(.type == "auditSummary")) | .[0].data.vulnerabilities.critical // 0' 2>/dev/null || echo "0")
        HIGH=$(echo "$AUDIT_OUTPUT" | jq -s 'map(select(.type == "auditSummary")) | .[0].data.vulnerabilities.high // 0' 2>/dev/null || echo "0")
        MOD=$(echo "$AUDIT_OUTPUT" | jq -s 'map(select(.type == "auditSummary")) | .[0].data.vulnerabilities.moderate // 0' 2>/dev/null || echo "0")
        LOW=$(echo "$AUDIT_OUTPUT" | jq -s 'map(select(.type == "auditSummary")) | .[0].data.vulnerabilities.low // 0' 2>/dev/null || echo "0")
        
        echo "    Critical: $CRIT | High: $HIGH | Moderate: $MOD | Low: $LOW"
        
        TOTAL_VULNERABILITIES=$((TOTAL_VULNERABILITIES + VULN_COUNT))
        CRITICAL_COUNT=$((CRITICAL_COUNT + CRIT))
        HIGH_COUNT=$((HIGH_COUNT + HIGH))
        MODERATE_COUNT=$((MODERATE_COUNT + MOD))
        LOW_COUNT=$((LOW_COUNT + LOW))
        
        FAILED_AUDITS+=("$name (yarn)")
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && yarn audit")
        MANUAL_COMMANDS+=("# To fix (add resolutions to package.json):")
        MANUAL_COMMANDS+=("cd $dir && yarn audit --json | jq -r '.data.advisory | select(.module_name != null) | \"  \\(.module_name)@^\\(.findings[0].version)\"' # Add these to resolutions section")
        MANUAL_COMMANDS+=("")
    else
        echo -e "  ${GREEN}✓ No vulnerabilities found${NC}"
    fi
}

# Function to audit npm packages
audit_npm() {
    local dir=$1
    local name=$2
    
    if [ ! -f "$dir/package.json" ] || [ ! -f "$dir/package-lock.json" ]; then
        return
    fi
    
    echo -e "${YELLOW}📦 Auditing (npm): $name${NC}"
    cd "$dir"
    
    if ! command_exists npm; then
        echo -e "  ${RED}✗ ERROR: npm not found${NC}"
        ERRORS+=("npm not found - cannot audit $name")
        FAILED_AUDITS+=("$name (npm)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && npm audit")
        MANUAL_COMMANDS+=("# To fix automatically:")
        MANUAL_COMMANDS+=("cd $dir && npm audit fix")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    AUDIT_OUTPUT=$(npm audit --json 2>&1)
    AUDIT_EXIT_CODE=$?
    
    if [ $AUDIT_EXIT_CODE -ne 0 ] && ! echo "$AUDIT_OUTPUT" | jq -e '.metadata' >/dev/null 2>&1; then
        echo -e "  ${RED}✗ ERROR: npm audit failed${NC}"
        ERRORS+=("npm audit failed for $name")
        FAILED_AUDITS+=("$name (npm)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && npm audit")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    VULN_COUNT=$(echo "$AUDIT_OUTPUT" | jq '.metadata.vulnerabilities | to_entries | map(.value) | add // 0' 2>/dev/null || echo "0")
    
    if [ "$VULN_COUNT" -gt 0 ]; then
        echo -e "  ${RED}✗ Found $VULN_COUNT vulnerabilities${NC}"
        
        CRIT=$(echo "$AUDIT_OUTPUT" | jq '.metadata.vulnerabilities.critical // 0' 2>/dev/null || echo "0")
        HIGH=$(echo "$AUDIT_OUTPUT" | jq '.metadata.vulnerabilities.high // 0' 2>/dev/null || echo "0")
        MOD=$(echo "$AUDIT_OUTPUT" | jq '.metadata.vulnerabilities.moderate // 0' 2>/dev/null || echo "0")
        LOW=$(echo "$AUDIT_OUTPUT" | jq '.metadata.vulnerabilities.low // 0' 2>/dev/null || echo "0")
        
        echo "    Critical: $CRIT | High: $HIGH | Moderate: $MOD | Low: $LOW"
        
        TOTAL_VULNERABILITIES=$((TOTAL_VULNERABILITIES + VULN_COUNT))
        CRITICAL_COUNT=$((CRITICAL_COUNT + CRIT))
        HIGH_COUNT=$((HIGH_COUNT + HIGH))
        MODERATE_COUNT=$((MODERATE_COUNT + MOD))
        LOW_COUNT=$((LOW_COUNT + LOW))
        
        FAILED_AUDITS+=("$name (npm)")
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && npm audit")
        MANUAL_COMMANDS+=("# To fix automatically:")
        MANUAL_COMMANDS+=("cd $dir && npm audit fix")
        MANUAL_COMMANDS+=("# To fix with breaking changes:")
        MANUAL_COMMANDS+=("cd $dir && npm audit fix --force")
        MANUAL_COMMANDS+=("")
    else
        echo -e "  ${GREEN}✓ No vulnerabilities found${NC}"
    fi
}

# Function to audit Cargo/Rust packages
audit_cargo() {
    local dir=$1
    local name=$2
    
    if [ ! -f "$dir/Cargo.toml" ]; then
        return
    fi
    
    echo -e "${YELLOW}🦀 Auditing: $name${NC}"
    cd "$dir"
    
    if ! command_exists cargo; then
        echo -e "  ${RED}✗ ERROR: cargo not found${NC}"
        ERRORS+=("cargo not found - cannot audit $name")
        FAILED_AUDITS+=("$name (cargo)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && cargo audit")
        MANUAL_COMMANDS+=("# Note: cargo audit requires 'cargo install cargo-audit'")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    # Check if cargo-audit is installed
    if ! cargo audit --version >/dev/null 2>&1; then
        echo -e "  ${RED}✗ ERROR: cargo-audit not installed${NC}"
        ERRORS+=("cargo-audit not installed - cannot audit $name")
        FAILED_AUDITS+=("$name (cargo)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# Install cargo-audit first:")
        MANUAL_COMMANDS+=("cargo install cargo-audit")
        MANUAL_COMMANDS+=("# Then audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && cargo audit")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    AUDIT_OUTPUT=$(cargo audit --json 2>&1)
    AUDIT_EXIT_CODE=$?
    
    if [ $AUDIT_EXIT_CODE -ne 0 ] && ! echo "$AUDIT_OUTPUT" | jq -e '.vulnerabilities' >/dev/null 2>&1; then
        echo -e "  ${RED}✗ ERROR: cargo audit failed${NC}"
        ERRORS+=("cargo audit failed for $name")
        FAILED_AUDITS+=("$name (cargo)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && cargo audit")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    VULN_COUNT=$(echo "$AUDIT_OUTPUT" | jq '.vulnerabilities.list | length' 2>/dev/null || echo "0")
    
    if [ "$VULN_COUNT" -gt 0 ]; then
        echo -e "  ${RED}✗ Found $VULN_COUNT vulnerabilities${NC}"
        
        # Parse severity from cargo audit (if available)
        # Cargo audit provides severity in each vulnerability entry
        CRIT=$(echo "$AUDIT_OUTPUT" | jq '[.vulnerabilities.list[] | select(.advisory.severity == "critical")] | length' 2>/dev/null || echo "0")
        HIGH=$(echo "$AUDIT_OUTPUT" | jq '[.vulnerabilities.list[] | select(.advisory.severity == "high")] | length' 2>/dev/null || echo "0")
        MOD=$(echo "$AUDIT_OUTPUT" | jq '[.vulnerabilities.list[] | select(.advisory.severity == "moderate" or .advisory.severity == "medium")] | length' 2>/dev/null || echo "0")
        LOW=$(echo "$AUDIT_OUTPUT" | jq '[.vulnerabilities.list[] | select(.advisory.severity == "low")] | length' 2>/dev/null || echo "0")
        
        echo "    Critical: $CRIT | High: $HIGH | Moderate: $MOD | Low: $LOW"
        
        TOTAL_VULNERABILITIES=$((TOTAL_VULNERABILITIES + VULN_COUNT))
        CRITICAL_COUNT=$((CRITICAL_COUNT + CRIT))
        HIGH_COUNT=$((HIGH_COUNT + HIGH))
        MODERATE_COUNT=$((MODERATE_COUNT + MOD))
        LOW_COUNT=$((LOW_COUNT + LOW))
        
        FAILED_AUDITS+=("$name (cargo)")
        MANUAL_COMMANDS+=("# To audit $name manually:")
        MANUAL_COMMANDS+=("cd $dir && cargo audit")
        MANUAL_COMMANDS+=("# To fix, update Cargo.toml dependencies or check:")
        MANUAL_COMMANDS+=("cd $dir && cargo audit --json | jq -r '.vulnerabilities.list[] | \"\\(.package.name) \\(.package.version) -> \\(.advisory.title)\"'")
        MANUAL_COMMANDS+=("")
    else
        echo -e "  ${GREEN}✓ No vulnerabilities found${NC}"
    fi
}

# Function to audit Maven/Java packages
audit_maven() {
    local dir=$1
    local name=$2
    
    if [ ! -f "$dir/pom.xml" ]; then
        return
    fi
    
    echo -e "${YELLOW}☕ Auditing: $name${NC}"
    cd "$dir"
    
    if ! command_exists mvn; then
        echo -e "  ${RED}✗ ERROR: mvn not found${NC}"
        ERRORS+=("mvn not found - cannot audit $name")
        FAILED_AUDITS+=("$name (maven)")
        ERROR_COUNT=$((ERROR_COUNT + 1))
        MANUAL_COMMANDS+=("# To audit $name manually (requires Maven):")
        MANUAL_COMMANDS+=("cd $dir && mvn org.owasp:dependency-check-maven:check")
        MANUAL_COMMANDS+=("")
        return
    fi
    
    # Run Maven OWASP dependency-check
    # We use the plugin directly without requiring it in pom.xml
    echo "  Running OWASP dependency-check (this may take a while on first run)..."
    MAVEN_OUTPUT=$(mvn org.owasp:dependency-check-maven:check -DskipTests 2>&1)
    MAVEN_EXIT_CODE=$?
    
    if [ $MAVEN_EXIT_CODE -ne 0 ]; then
        # Check if it's a plugin configuration issue or actual failure
        if echo "$MAVEN_OUTPUT" | grep -q "BUILD SUCCESS"; then
            MAVEN_EXIT_CODE=0
        else
            echo -e "  ${RED}✗ ERROR: Maven dependency-check failed${NC}"
            ERRORS+=("Maven dependency-check failed for $name")
            FAILED_AUDITS+=("$name (maven)")
            ERROR_COUNT=$((ERROR_COUNT + 1))
            MANUAL_COMMANDS+=("# To audit $name manually:")
            MANUAL_COMMANDS+=("cd $dir && mvn org.owasp:dependency-check-maven:check")
            MANUAL_COMMANDS+=("# Or add to pom.xml and run:")
            MANUAL_COMMANDS+=("cd $dir && mvn verify")
            MANUAL_COMMANDS+=("")
            return
        fi
    fi
    
    # Check for the report
    REPORT_FILE="$dir/target/dependency-check-report.html"
    JSON_REPORT="$dir/target/dependency-check-report.json"
    
    if [ -f "$JSON_REPORT" ]; then
        # Parse JSON report for vulnerability count
        VULN_COUNT=$(jq '.dependencies | map(.vulnerabilities // []) | flatten | length' "$JSON_REPORT" 2>/dev/null || echo "0")
        
        if [ "$VULN_COUNT" -gt 0 ]; then
            echo -e "  ${RED}✗ Found $VULN_COUNT vulnerabilities${NC}"
            
            # Try to parse severities from JSON report
            CRIT=$(jq '[.dependencies[].vulnerabilities[]? | select(.severity == "CRITICAL")] | length' "$JSON_REPORT" 2>/dev/null || echo "0")
            HIGH=$(jq '[.dependencies[].vulnerabilities[]? | select(.severity == "HIGH")] | length' "$JSON_REPORT" 2>/dev/null || echo "0")
            MOD=$(jq '[.dependencies[].vulnerabilities[]? | select(.severity == "MEDIUM")] | length' "$JSON_REPORT" 2>/dev/null || echo "0")
            LOW=$(jq '[.dependencies[].vulnerabilities[]? | select(.severity == "LOW")] | length' "$JSON_REPORT" 2>/dev/null || echo "0")
            
            echo "    Critical: $CRIT | High: $HIGH | Moderate: $MOD | Low: $LOW"
            echo "    Report: $REPORT_FILE"
            
            TOTAL_VULNERABILITIES=$((TOTAL_VULNERABILITIES + VULN_COUNT))
            CRITICAL_COUNT=$((CRITICAL_COUNT + CRIT))
            HIGH_COUNT=$((HIGH_COUNT + HIGH))
            MODERATE_COUNT=$((MODERATE_COUNT + MOD))
            LOW_COUNT=$((LOW_COUNT + LOW))
            
            FAILED_AUDITS+=("$name (maven)")
            MANUAL_COMMANDS+=("# To audit $name manually:")
            MANUAL_COMMANDS+=("cd $dir && mvn org.owasp:dependency-check-maven:check")
            MANUAL_COMMANDS+=("# View report:")
            MANUAL_COMMANDS+=("xdg-open $REPORT_FILE  # or open $REPORT_FILE on macOS")
            MANUAL_COMMANDS+=("# To fix, update dependencies in pom.xml")
            MANUAL_COMMANDS+=("")
        else
            echo -e "  ${GREEN}✓ No vulnerabilities found${NC}"
        fi
    elif [ -f "$REPORT_FILE" ]; then
        # Fallback: HTML report exists but no JSON
        echo -e "  ${YELLOW}⚠ Check $REPORT_FILE for details${NC}"
        echo -e "  ${YELLOW}  (JSON report not generated, cannot parse vulnerability count)${NC}"
        MANUAL_COMMANDS+=("# To view $name report:")
        MANUAL_COMMANDS+=("xdg-open $REPORT_FILE  # or open $REPORT_FILE on macOS")
        MANUAL_COMMANDS+=("")
    else
        echo -e "  ${GREEN}✓ No vulnerabilities found${NC}"
    fi
}

# Change to workspace root
cd /workspaces/step

# ============================================================================
# SECTION 1: Audit main packages directory (Yarn workspace)
# ============================================================================
print_section "JavaScript/TypeScript Packages (Yarn)"

audit_yarn "/workspaces/step/packages" "packages (workspace root)"
audit_yarn "/workspaces/step/packages/ui-core" "ui-core"
audit_yarn "/workspaces/step/packages/ui-essentials" "ui-essentials"
audit_yarn "/workspaces/step/packages/voting-portal" "voting-portal"
audit_yarn "/workspaces/step/packages/admin-portal" "admin-portal"
audit_yarn "/workspaces/step/packages/ballot-verifier" "ballot-verifier"

# ============================================================================
# SECTION 2: Audit browserstack directories (npm)
# ============================================================================
print_section "Browserstack Packages (npm)"

audit_npm "/workspaces/step/packages/strand/browserstack" "strand/browserstack"
audit_npm "/workspaces/step/packages/sequent-core/browserstack" "sequent-core/browserstack"
audit_npm "/workspaces/step/packages/ballot-verifier/browserstack" "ballot-verifier/browserstack"

# ============================================================================
# SECTION 3: Audit docs/api
# ============================================================================
print_section "Documentation API Packages"

audit_yarn "/workspaces/step/docs/api" "docs/api"
audit_yarn "/workspaces/step/docs/api/graphql" "docs/api/graphql"

# ============================================================================
# SECTION 4: Audit Rust packages (Cargo)
# ============================================================================
print_section "Rust Packages (Cargo)"

# Note: cargo audit checks the entire workspace from the root, so we only need to run it once
audit_cargo "/workspaces/step/packages" "Cargo workspace (all Rust packages)"

# ============================================================================
# SECTION 5: Audit Java packages (Maven)
# ============================================================================
print_section "Java Packages (Maven)"

audit_maven "/workspaces/step/packages/keycloak-extensions" "keycloak-extensions"

# ============================================================================
# SUMMARY
# ============================================================================
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  AUDIT SUMMARY${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""

# Print errors if any
if [ $ERROR_COUNT -gt 0 ]; then
    echo -e "${RED}Errors encountered: $ERROR_COUNT${NC}"
    echo ""
    for error in "${ERRORS[@]}"; do
        echo -e "  ${RED}• $error${NC}"
    done
    echo ""
fi

# Print vulnerability summary
if [ $TOTAL_VULNERABILITIES -eq 0 ] && [ $ERROR_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓ No vulnerabilities found across all packages!${NC}"
    echo ""
    exit 0
else
    if [ $TOTAL_VULNERABILITIES -gt 0 ]; then
        echo -e "${RED}Total Vulnerabilities: $TOTAL_VULNERABILITIES${NC}"
        echo ""
        echo -e "  ${RED}Critical: $CRITICAL_COUNT${NC}"
        echo -e "  ${YELLOW}High:     $HIGH_COUNT${NC}"
        echo -e "  ${YELLOW}Moderate: $MODERATE_COUNT${NC}"
        echo -e "  ${BLUE}Low:      $LOW_COUNT${NC}"
        echo ""
        
        # List failed audits
        if [ ${#FAILED_AUDITS[@]} -gt 0 ]; then
            echo -e "${YELLOW}Packages with vulnerabilities or errors:${NC}"
            for pkg in "${FAILED_AUDITS[@]}"; do
                echo -e "  • $pkg"
            done
            echo ""
        fi
    fi
    
    # Print manual commands if there are any failures or vulnerabilities
    if [ ${#MANUAL_COMMANDS[@]} -gt 0 ]; then
        echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"
        echo -e "${MAGENTA}  MANUAL AUDIT & FIX COMMANDS${NC}"
        echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"
        echo ""
        echo -e "${YELLOW}Copy and paste these commands to audit/fix manually:${NC}"
        echo ""
        for cmd in "${MANUAL_COMMANDS[@]}"; do
            echo -e "$cmd"
        done
    fi
    
    echo -e "${YELLOW}⚠ Please review and fix the vulnerabilities above.${NC}"
    echo ""
    exit 1
fi
