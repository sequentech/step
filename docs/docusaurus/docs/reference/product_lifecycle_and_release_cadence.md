---
id: product_lifecycle_and_release_cadence
title: Product lifecycle and release cadence
---

# Sequent Voting Platform (SVP) Product Lifecycle and Release Cadence

The Sequent Voting Platform follows a predictable release cadence designed to
provide stability for enterprise deployments while enabling continuous
innovation and security updates.

*This release schedule is subject to change based on security requirements,
critical bug fixes, or significant architectural updates. Any changes will be
communicated in advance to enterprise customers.*

## Release Philosophy

SVP uses **YY.MM versioning** format for all releases:
- **YY**: Two-digit year (e.g., 25 for 2025)
- **MM**: Two-digit month (e.g., 09 for September)
- **PATCH**: Patch number for bug fixes and security updates (e.g., 25.09.1)

Major releases may contain breaking changes or significant architectural updates, while Minor releases are backward compatible feature additions. All releases follow the same YY.MM numbering scheme, with the release type determined by the release month and cadence.

## Release Types

### Major Releases

Major releases are enterprise-grade releases designed for production environments
requiring maximum stability and extended support. **Major releases may contain breaking changes** that require careful migration planning and testing.

- **Cadence**: Every 6 months (March and September)
- **Numbering**: YY.MM format (e.g., 25.09 for September 2025)
- **Breaking Changes**: May include API changes, database schema updates, or architectural modifications
- **Standard Major Release Support (SMRS)**: 2 months from release date
- **Extended Major Release Support (EMRS)**: Additional 4 months after standard support ends
- **Legacy Major Release Support (LMRS)**: Additional 6 months after extended support ends
- **Total Major Release Lifecycle**: 12 months (1 year)
- **Currently Supported Major Releases**: None
- **Next Major Release**: Version 25.09 (September 1st, 2025)

### Minor Releases

Minor releases provide the latest features and improvements for development
and testing environments, and can be also in special cases where a feature is
required in a short period of time. **Minor releases are backward compatible** and do not contain breaking changes.

- **Cadence**: Monthly releases
- **Numbering**: YY.MM format (e.g., 25.08 for August 2025)
- **Backward Compatibility**: All changes are backward compatible
- **Standard Minor Release Support (SMRS)**: 2 months from release date
- **Extended Minor Release Support (EMRS)**: Additional 4 months after standard
  support ends
- **Total Minor Release Lifecycle**: 6 months
- **Currently Supported Minor Releases**: None
- **Next Minor Release**: Version 25.08 (August 1st, 2025)



## Release Schedule Table

| Version    | Release Date | Release Type | Standard Support Until | Extended Support Until | Legacy Support Until | Total Support |
|------------|-------------|---------|-----------------------|------------------------|---------------------|---------------|
| 25.08      | Aug 1, 2025 | Minor   | Oct 1, 2025           | Feb 1, 2026            | -                   | 6 months      |
| **25.09**  | Sep 1, 2025 | **Major** | Nov 1, 2025           | Mar 1, 2026            | Sep 1, 2026         | **12 months** |
| 25.10      | Oct 1, 2025 | Minor   | Dec 1, 2025           | Apr 1, 2026            | -                   | 6 months      |
| 25.11      | Nov 1, 2025 | Minor   | Jan 1, 2026           | May 1, 2026            | -                   | 6 months      |
| 25.12      | Dec 1, 2025 | Minor   | Feb 1, 2026           | Jun 1, 2026            | -                   | 6 months      |
| 26.01      | Jan 1, 2026 | Minor   | Mar 1, 2026           | Jul 1, 2026            | -                   | 6 months      |
| 26.02      | Feb 1, 2026 | Minor   | Apr 1, 2026           | Aug 1, 2026            | -                   | 6 months      |
| **26.03**  | Mar 1, 2026 | **Major** | May 1, 2026           | Sep 1, 2026            | Mar 1, 2027         | **12 months** |
| 26.04      | Apr 1, 2026 | Minor   | Jun 1, 2026           | Oct 1, 2026            | -                   | 6 months      |
| 26.05      | May 1, 2026 | Minor   | Jul 1, 2026           | Nov 1, 2026            | -                   | 6 months      |
| 26.06      | Jun 1, 2026 | Minor   | Aug 1, 2026           | Dec 1, 2026            | -                   | 6 months      |
| 26.07      | Jul 1, 2026 | Minor   | Sep 1, 2026           | Jan 1, 2027            | -                   | 6 months      |
| 26.08      | Aug 1, 2026 | Minor   | Oct 1, 2026           | Feb 1, 2027            | -                   | 6 months      |
| **26.09**  | Sep 1, 2026 | **Major** | Nov 1, 2026           | Mar 1, 2027            | Sep 1, 2027         | **12 months** |
| 26.10      | Oct 1, 2026 | Minor   | Dec 1, 2026           | Apr 1, 2027            | -                   | 6 months      |
| 26.11      | Nov 1, 2026 | Minor   | Jan 1, 2027           | May 1, 2027            | -                   | 6 months      |
| 26.12      | Dec 1, 2026 | Minor   | Feb 1, 2027           | Jun 1, 2027            | -                   | 6 months      |
| 27.01      | Jan 1, 2027 | Minor   | Mar 1, 2027           | Jul 1, 2027            | -                   | 6 months      |
| 27.02      | Feb 1, 2027 | Minor   | Apr 1, 2027           | Aug 1, 2027            | -                   | 6 months      |
| **27.03**  | Mar 1, 2027 | **Major** | May 1, 2027           | Sep 1, 2027            | Mar 1, 2028         | **12 months** |

## Support Levels

### Community Support

Free community support is available via GitHub tickets and our Discord channel,
with no SLA guarantees and absolutely no warranty.

### Enterprise Support

Enterprise customers receive:
- Priority support during standard support period
- Standard, Extended and Legacy support options
- Migration assistance between major and minor versions
- Custom support agreements for extended lifecycles
- Dedicated support channels

#### Standard Support (SMRS)

- Security patches and critical bug fixes
- Technical support through official channels
- Documentation updates
- Community support

#### Extended Support (EMRS)

*Available for all releases*

- Security patches and critical bug fixes
- Limited technical support through official channels
- Extended maintenance for enterprise customers
- Migration assistance to newer versions

#### Legacy Support (LMRS)

*Available for Major releases only*

- Security patches and critical bug fixes
- Limited technical support through official channels
- Priority migration assistance to newer Major versions
- Extended maintenance for enterprise customers



## Release Timeline Visualization

```mermaid
---
config:
  theme: 'default'
  themeVariables:
    cScale0: '#0f054c'
    cScale1: '#2de8b9'
---
timeline
    title SVP Release Schedule
    
    section 2025
        Sep 1 : 25.09 Major : Major Release
        Oct 1 : 25.10 Minor
        Nov 1 : 25.11 Minor
        Dec 1 : 25.12 Minor
    
    section 2026
        Jan 1 : 26.01 Minor
        Feb 1 : 26.02 Minor
        Mar 1 : 26.03 Major : Major Release
        Apr 1 : 26.04 Minor
```

## Support Lifecycle Visualization

### Release Support Timeline

```mermaid
---
displayMode: compact
config:
    logLevel: 'debug'
    theme: 'default'
    themeCSS: " \n
        .taskText { font-size: 16px; }
        rect[id^=smrs_] { fill: #0f054c; stroke-width: 4px; }
        text[id^=smrs_] { fill: white !important; font-size: 24px; }
        rect[id^=emrs_] { fill: #2de8b9; stroke-width: 4px; }
        text[id^=emrs_] { fill: #0f054c !important; font-size: 24px; }
        rect[id^=lmrs_] { fill: #bec7ff; stroke-width: 4px; }
        text[id^=lmrs_] { fill: #0f054c !important; font-size: 24px; }
        .sectionTitle { stroke: white; paint-order: stroke fill; fill: #0f054c; stroke-width: 8px; }
        g[class=tick] text { font-size: 24px; height: 50px; }
        .vertText {  transform: translate(-90px, -650px); font-size: 24px; fill: red !important; }
        .task.vert { stroke: red; fill: red !important; }

        /*** section backgrounds: ***/

        /* - under extended or legacy support only */
        .section0, .section1, .section2, .section3 { fill: #fff400; opacity: 0.2; stroke: none; }
        .section0:nth-last-child(-n + 2) { fill: transparent; }

        /* - under standard support */
        .section4 { fill: #6666ff7d; }
        #smrs_2512 { stroke: #0f054c; }

        /* out of support: */
        #smrs_2508, #smrs_2509, #smrs_2510, #smrs_2511 { stroke: red; fill: #0f054c; opacity: 0.2; }
        #smrs_2508-text, #smrs_2509-text, #smrs_2510-text, #smrs_2511-text { fill: #0f054c !important; }

        /* under legacy or extended support: */
        #emrs_2508, #emrs_2509, #lmrs_2509, #emrs_2510, #emrs_2511, #emrs_2512 { stroke: #0f054c; }
    "
---
%%{init:
    {
        "gantt": {
            "sectionFontSize": 24,
            "fontSize": 36,
            "barGap": 40,
            "barHeight": 50,
            "topPadding": 40
        }
    }
}%%
gantt
    todayMarker off
    dateFormat YYYY-MM-DD
    axisFormat %b %Y
    tickInterval 3month

    section 25.08
        Standard :done, smrs_2508, 2025-08-01, 61d
        Extended :done, emrs_2508, after smrs_2508, 122d

    section 25.09 Major
        Standard :done, smrs_2509, 2025-09-01, 61d
        Extended :done, emrs_2509, after smrs_2509, 122d
        Legacy   :active, lmrs_2509, after emrs_2509, 183d

    section 25.10
        Standard :done, smrs_2510, 2025-10-01, 61d
        Extended :done, emrs_2510, after smrs_2510, 122d

    section 25.11
        Standard :done, smrs_2511, 2025-11-01, 61d
        Extended :done, emrs_2511, after smrs_2511, 122d

    section 25.12
        Standard :done, smrs_2512, 2025-12-01, 61d
        Extended :done, emrs_2512, after smrs_2512, 122d

    Current Date : vert, current, 2026-01-17, 1d
```

In the example shown in the diagram (with `Current date` set to January 17, 2026):

**Major Releases:**
- **Version 25.09 Major** (released September 1, 2025): Currently in **Extended Major Release Support** phase, having completed its 2-month standard support in November 1, 2025. Extended supports continues until March 1, 2025. Legacy support continues until September 1, 2026.

**Minor Releases:**
- **Version 25.08 Minor** (released August 1, 2025): Currently in **Extended Minor Release Support** phase, having completed its 2-month standard support period. Extended support continues until February 1, 2026.
- **Version 25.10 Minor** (released October 1, 2025): Currently in **Extended Minor Release Support** phase, having completed its 2-month standard support period. Extended support continues until April 1, 2026.
- **Version 25.11 Minor** (released November 1, 2025): Currently in **Extended Minor Release Support** phase, having completed its 2-month standard support period. Extended support continues until May 1, 2026.
- **Version 25.12 Minor** (released December 1, 2025): Currently in **Standard Minor Release Support** phase, with full support continuing until February 1, 2026, then extended support until June 1, 2026.

The diagram illustrates the overlapping support windows that provide enterprise customers with migration flexibility. Major releases have a three-tier support model (Standard → Extended → Legacy) with 12 months of total support coverage, while Minor releases have a two-tier model (Standard → Extended) with 6 months of total support coverage.

## Feature Release Process

Each feature release follows this schedule:

1. **Feature Freeze**: 1 month before release date
2. **Beta Release**: 2 weeks before release date  
3. **Release Candidate**: 1 week before release date
4. **Final Release**: On scheduled date

## Security and Patch Updates

- **Security patches**: Released as needed for all supported versions
- **Regular patches**: Bi-weekly review cycle for dependencies
- **Emergency patches**: Released within 24-48 hours for critical security issues

## Minor Version Release Lifecycle

Each minor version follows a structured release process that includes
pre-releases, the final release, and subsequent patch releases during its
support lifecycle. This section illustrates the complete lifecycle of a single
minor version from initial development to end of support.

### Version 26.09.x Series Release Timeline (Example)

```mermaid
---
config:
    logLevel: 'debug'
    theme: 'default'
    themeCSS: " \n
        .taskText { font-size: 14px; font-weight: 500; }
        rect[id^=feat_dev] { fill: #2de8b9; stroke-width: 3px; stroke: #2de8b9; }
        text[id^=feat_dev] { fill: #0f054c !important; font-size: 20px; font-weight: 600; }
        text[id^=feat_blank] { opacity: 0; }
        rect[id^=rc_] { fill: #ff9500; stroke-width: 3px; stroke: #cc7700; }
        text[id^=rc_] { fill: #0f054c !important; font-size: 20px; font-weight: 600; }
        rect[id^=final_] { fill: #0f054c; stroke-width: 4px; stroke: #0a0339; }
        text[id^=final_] { fill: #0f054c !important; font-size: 22px; font-weight: 700; }
        rect[id^=patch_] { fill: #2de8b9; stroke-width: 3px; stroke: #24c7a0; }
        text[id^=patch_] { fill: #0f054c !important; font-size: 20px; font-weight: 600; }
        rect[id^=security_] { fill: #e63946; stroke-width: 4px; stroke: #d62828; }
        text[id^=security_] { fill: #0f054c !important; font-size: 20px; font-weight: 700; }
        .sectionTitle { stroke: white; paint-order: stroke fill; fill: #0f054c; stroke-width: 8px; font-size: 28px; }
        g[class=tick] text { font-size: 18px; }

        /*** section backgrounds ***/
        .section0 { fill: #2de8b9; opacity: 0.2; }
        .section1 { fill: #ff9500; opacity: 0.2; }
        .section2 { fill: #6666ff7d; opacity: 0.5; }
        .section3 { fill: #fff400; opacity: 0.2; stroke: none; }
        .section4 { fill: #ffcccc; opacity: 0.3; }
    "
---
%%{init:
    {
        "gantt": {
            "sectionFontSize": 26,
            "fontSize": 24,
            "barGap": 35,
            "barHeight": 45,
            "topPadding": 70
        }
    }
}%%
gantt
    dateFormat  YYYY-MM-DD
    axisFormat  %b %Y
    tickInterval 3month
    
    section Feature Development Phase
    Feature Development     :done, feat_dev, 2026-04-01, 90d
    _                        :done, feat_blank, 2026-04-01, 0
    
    section Feature Freeze Phase
    Release Candidate 0     :done, rc_0, 2026-07-01, 30d
    Release Candidate 1     :done, rc_1, after rc_0, 15d
    Release Candidate 2     :done, rc_2, after rc_1, 15d
    Final Release           :done, final_release, after rc_2, 7d

    section Standard Support Phase
    Version 26.09.0         :milestone, 2026-09-02, 0d
    Bugfix Release 26.09.1  :done, patch_1, 2026-10-15, 1d
    Final Standard Patch 26.09.2 :active, patch_2, 2026-10-30, 1d
    
    section Extended Support Phase
    Security Release 26.09.3 :crit, sec3, 2026-12-15, 1d
    Critical Patch 26.09.4   :crit, patch_4, 2027-01-15, 1d
    Final Extended Patch 26.09.5 :active, patch_5, 2027-02-15, 1d
    
    section Legacy Support Phase
    Security Only 26.09.6    :crit, sec6, 2027-05-01, 1d
    Legacy Security 26.09.7   :crit, sec7, 2027-07-01, 1d
    EOL Security 26.09.8     :crit, sec8, 2027-08-15, 1d
```

### Release Details Table (Example)

| Release | Release Date | Type | Purpose & Rationale |
|---------|-------------|------|-------------------|
| **Feature Development** | Apr 1 - Jun 30, 2026 | Development Phase | Active feature development period for Major release 26.09. New features, API enhancements, and architectural improvements. Breaking changes allowed during this phase. |
| **26.09.0-rc.0** | Jul 1, 2026 | Release Candidate | **Feature Freeze Phase**: Initial release candidate for community testing. Major features freeze completed. Focus on stability testing and performance validation. |
| **26.09.0-rc.1** | Aug 1, 2026 | Release Candidate | **Feature Freeze Phase**: Second release candidate addressing critical bugs found in rc.0. Database migration optimizations and API refinements. |
| **26.09.0-rc.2** | Aug 15, 2026 | Release Candidate | **Feature Freeze Phase**: Third release candidate for final testing. Documentation finalization and UI/UX polish. Performance benchmarking completed. |
| **26.09.0** | Sep 1, 2026 | **Major Final** | **Official Major release**. All quality gates passed. Production-ready with full documentation, security audit completed. |
| **26.09.1** | Oct 15, 2026 | Bugfix Patch | **Standard Support Phase**: Address non-critical bugs reported in production: memory leak in vote processing, timezone handling issues, minor UI inconsistencies. |
| **26.09.2** | Oct 30, 2026 | Final Standard Patch | **Standard Support Phase**: Last scheduled patch during standard support. Includes final compatibility updates and minor stability enhancements before transitioning to Extended Support. |
| **26.09.3** | Dec 15, 2026 | Security Patch | **Extended Support Phase**: Critical security update addressing privilege escalation vulnerability. Updated cryptographic libraries and enhanced input validation. |
| **26.09.4** | Jan 15, 2027 | Critical Patch | **Extended Support Phase**: Emergency fix for vote tallying algorithm edge case discovered in large-scale elections. Includes performance optimizations for high-concurrency scenarios. |
| **26.09.5** | Feb 15, 2027 | Final Extended Patch | **Extended Support Phase**: Last scheduled patch during extended support. Final compatibility updates and documentation improvements before transitioning to Legacy Support. |
| **26.09.6** | May 1, 2027 | Legacy Security | **Legacy Support Phase**: Security-only patch addressing newly discovered authentication bypass vulnerability. Limited support scope, security patches only. |
| **26.09.7** | Jul 1, 2027 | Legacy Security | **Legacy Support Phase**: Critical security fix for zero-day vulnerability affecting authentication systems. Enterprise migration assistance provided. |
| **26.09.8** | Aug 15, 2027 | End-of-Life Security | **Legacy Support Phase**: Final security patch before end-of-life. Last critical security fix. End of support announced for Sep 1, 2027. |

### Release Process Timeline

Before diving into a specific example, it's important to understand the conceptual framework that governs all minor version releases. This process ensures quality, stability, and predictable timing for enterprise customers.

#### Mandatory Release Process Flow

```mermaid
---
config:
    logLevel: 'debug'
    theme: 'default'
    themeVariables:
        cScale0: '#0f054c'
        cScale1: '#2de8b9'
---
flowchart TD
    A[Feature Development] --> B[Feature Freeze]
    B --> C[Release Candidate 0]
    C --> D{Testing & <br/>Bug Fixes}
    D -->|Major Issues Found| E[Release Candidate N+1]
    E --> D
    D -->|Ready for Release| F[Final Release Candidate]
    F --> G[2 Week Mandatory<br/>Stabilization Period]
    G --> H[Major Final Release]
    H --> I[Production Support Begins]
    
    style A fill:#4a90e2,stroke:#3a7bc8,color:#fff
    style B fill:#ff9500,stroke:#cc7700,color:#fff
    style C fill:#ff9500,stroke:#cc7700,color:#fff
    style E fill:#ff9500,stroke:#cc7700,color:#fff
    style F fill:#ff9500,stroke:#cc7700,color:#fff
    style G fill:#e63946,stroke:#d62828,color:#fff
    style H fill:#0f054c,stroke:#0a0339,color:#fff
    style I fill:#2de8b9,stroke:#24c7a0,color:#0f054c
```

#### Timing Requirements

| Phase | Duration | Description | Mandatory Wait |
|-------|----------|-------------|----------------|
| **Feature Development** | 3-6 months | Active development, new features, breaking changes allowed | No mandatory wait |
| **Feature Freeze to RC.0** | 2-4 weeks | Code stabilization, initial testing | No mandatory wait |
| **Between Release Candidates** | 1-2 weeks | Bug fixes, regression testing | Minimum 1 week |
| **Final RC to Major Release** | **2 weeks** | **Mandatory stabilization period** | **Exactly 2 weeks** |
| **Post-Release Monitoring** | 2-4 weeks | Production stability validation | N/A |

#### Critical Rules

1. **Feature Development Phase**: During this phase, new features are actively developed and breaking changes are allowed. This phase typically lasts 3-6 months depending on the scope of the Major release.

2. **Feature Freeze**: All new features must be code-complete and merged before the feature freeze deadline. Only bug fixes and stabilization work are allowed after this point.

3. **Mandatory 2-Week Period**: There must be exactly 2 weeks between the final release candidate and the Major release. This is non-negotiable and allows for:
   - Final security audits
   - Documentation review and finalization
   - Community feedback integration
   - Infrastructure preparation for release

2. **Release Candidate Progression**: Each release candidate must be available for at least 1 week before the next RC or final release.

3. **No Direct-to-Production**: All Major releases must go through at least one release candidate phase.

4. **Emergency Exception Process**: In case of critical security vulnerabilities, the 2-week period may be shortened to 1 week with explicit approval from the security team and release management.

