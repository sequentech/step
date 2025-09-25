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

SVP uses **Major.Minor.Patch versioning** format for all releases:
- **Major**: Increments for releases with significant breaking changes (e.g., 9, 10).
- **Minor**: Increments for monthly releases, resetting to 0 for new Major releases (e.g., 9.1, 9.2).
- **Patch**: Increments for bug fixes and security updates (e.g., 9.0.1, 9.1.1).

Major releases may contain breaking changes or significant architectural
updates, while Minor releases are backward compatible feature additions. All
releases follow the same Major.Minor.Patch numbering scheme, with the release type
determined by the release month and cadence.

## Release Types

### Major Releases

Major releases are enterprise-grade releases designed for production
environments requiring maximum stability and extended support. **Major releases
may contain breaking changes** that require careful migration planning and
testing.

- **Cadence**: Every 6 months (March and September)
- **Numbering**: Major.0 (e.g., 9.0 for the September 2025 release)
- **Breaking Changes**: May include API changes, database schema updates, or
  architectural modifications
- **Long Term Support (LTS)**: 8 months from release date
- **Total Major Release Lifecycle**: 8 months
- **Currently Supported Major Releases**: None
- **Next Major Release**: Version 9.0 (September 1st, 2025)

### Minor Releases

Minor releases provide the latest features and improvements for development and
testing environments, and can also be used in special cases where a feature is
required in a short period of time. **Minor releases are backward compatible**
and do not contain breaking changes.

- **Cadence**: Monthly releases
- **Numbering**: Major.Minor (e.g., 9.1 for October 2025)
- **Backward Compatibility**: All changes are backward compatible
- **Standard Release Support (SRS)**: 2 months from release date
- **Extended Release Support (ERS)**: Additional 2 months after standard
  support ends
- **Total Minor Release Lifecycle**: 4 months
- **Currently Supported Minor Releases**: None
- **Next Minor Release**: Version 9.1 (October 1st, 2025)



## Release Schedule Table

| Version    | Release Date | Release Type | Long Term Support Until | Extended Support Until | Legacy Support Until | Total Support |
|------------|-------------|---------|-----------------------|------------------------|---------------------|---------------|
| **9.0**    | Sep 1, 2025 | **Major** | May 1, 2026           | -                      | -                   | **8 months** |
| 9.1        | Oct 1, 2025 | Minor   | Dec 1, 2025           | Feb 1, 2026            | -                   | 4 months      |
| 9.2        | Nov 1, 2025 | Minor   | Jan 1, 2026           | Mar 1, 2026            | -                   | 4 months      |
| 9.3        | Dec 1, 2025 | Minor   | Feb 1, 2026           | Apr 1, 2026            | -                   | 4 months      |
| 9.4        | Jan 1, 2026 | Minor   | Mar 1, 2026           | May 1, 2026            | -                   | 4 months      |
| 9.5        | Feb 1, 2026 | Minor   | Apr 1, 2026           | Jun 1, 2026            | -                   | 4 months      |
| **10.0**   | Mar 1, 2026 | **Major** | Nov 1, 2026           | -                      | -                   | **8 months** |
| 10.1       | Apr 1, 2026 | Minor   | Jun 1, 2026           | Aug 1, 2026            | -                   | 4 months      |
| 10.2       | May 1, 2026 | Minor   | Jul 1, 2026           | Sep 1, 2026            | -                   | 4 months      |
| 10.3       | Jun 1, 2026 | Minor   | Aug 1, 2026           | Oct 1, 2026            | -                   | 4 months      |
| 10.4       | Jul 1, 2026 | Minor   | Sep 1, 2026           | Nov 1, 2026            | -                   | 4 months      |
| 10.5       | Aug 1, 2026 | Minor   | Oct 1, 2026           | Dec 1, 2026            | -                   | 4 months      |
| **11.0**   | Sep 1, 2026 | **Major** | May 1, 2027           | -                      | -                   | **8 months** |
| 11.1       | Oct 1, 2026 | Minor   | Dec 1, 2026           | Feb 1, 2027            | -                   | 4 months      |
| 11.2       | Nov 1, 2026 | Minor   | Jan 1, 2027           | Mar 1, 2027            | -                   | 4 months      |
| 11.3       | Dec 1, 2026 | Minor   | Feb 1, 2027           | Apr 1, 2027            | -                   | 4 months      |
| 11.4       | Jan 1, 2027 | Minor   | Mar 1, 2027           | May 1, 2027            | -                   | 4 months      |
| 11.5       | Feb 1, 2027 | Minor   | Apr 1, 2027           | Jun 1, 2027            | -                   | 4 months      |
| **12.0**   | Mar 1, 2027 | **Major** | Nov 1, 2027           | -                      | -                   | **8 months** |

## Support Levels

### Community Support

Free community support is available via GitHub tickets and our Discord channel,
with no SLA guarantees and absolutely no warranty.

### Enterprise Support

Enterprise customers receive:
- Dedicated support channels
- Security patches and critical bug fixes
- Standard, Extended and Legacy support options for Minor releases
- Long Term Support (LTS) for Major releases
- Migration assistance between major and minor versions
- Custom support agreements for extended lifecycles
- Documentation updates

Enterprise Support Plans:
- **Standard Support** (SRS):
  - Available for Minor releases
  - 2 months from release date
- **Extended Support** (ERS):
  - Available for Minor releases
  - Additional 2 months after Standard Support ends
- **Long Term Support** (LTS):
  - Available for Major releases only
  - 8 months from release date

## Release Timeline Visualization

```mermaid
---
config:
    theme: 'default'
    themeVariables:
        cScale0: '#0f054c'
        cScale1: '#2de8b9'
    themeCSS: " \n
        .timeline-node tspan { font-size: 24px; }
    "
---
timeline
    title SVP Release Schedule
    
    section 2025
        Sep 1 : 9.0 Major : Major Release
        Oct 1 : 9.1 Minor
        Nov 1 : 9.2 Minor
        Dec 1 : 9.3 Minor
    
    section 2026
        Jan 1 : 9.4 Minor
        Feb 1 : 9.5 Minor
        Mar 1 : 10.0 Major : Major Release
        Apr 1 : 10.1 Minor
```

## Support Lifecycle Visualization

### Release Support Timeline

*Note: The diagram below shows an example timeline with a hypothetical current date of January 17, 2026, to illustrate how different releases would be in various support phases.*

```mermaid
---
displayMode: compact
config:
    logLevel: 'debug'
    theme: 'default'
    themeCSS: " \n
        .taskText { font-size: 16px; }
        rect[id^=srs_] { fill: #0f054c; stroke-width: 4px; }
        text[id^=srs_] { fill: white !important; font-size: 24px; }
        rect[id^=ers_] { fill: #2de8b9; stroke-width: 4px; }
        text[id^=ers_] { fill: #0f054c !important; font-size: 24px; }
        rect[id^=lts_] { fill: #bec7ff; stroke-width: 4px; }
        text[id^=lts_] { fill: #0f054c !important; font-size: 24px; }
        .sectionTitle { stroke: white; paint-order: stroke fill; fill: #0f054c; stroke-width: 8px; }
        g[class=tick] text { font-size: 24px; height: 50px; }
        .vertText {  transform: translate(-133px, -740px); font-size: 24px; fill: red !important; }
        .task.vert { stroke: red; fill: red !important; }

        /*** section backgrounds: ***/
        .section0, .section1, .section2 { fill: #2de8b9; opacity: 0.2; stroke: none; }
        .section:nth-last-child(-n + 1) { fill: transparent; }

        /* - under standard support */
        .section3, .section4 { fill: #6666ff7d; }
        #srs_93, #srs_94 { stroke: #0f054c; }

        /* out of support: */
        /* (No versions are out of support in this example timeframe) */

        /* under legacy or extended support: */
        #lts_90, #ers_91, #ers_92 { stroke: #0f054c; }

        /* unreleased: */
        #srs_95, #ers_95 { opacity: 0.3; }
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
    tickInterval 2month

    section 9.0 Major
        Long Term Support :active, lts_90, 2025-09-01, 243d

    section 9.1
        Standard :done, srs_91, 2025-10-01, 61d
        Extended :active, ers_91, after srs_91, 61d

    section 9.2
        Standard :done, srs_92, 2025-11-01, 61d
        Extended :active, ers_92, after srs_92, 61d

    section 9.3
        Standard :active, srs_93, 2025-12-01, 61d
        Extended :ers_93, after srs_93, 61d

    section 9.4
        Standard :active, srs_94, 2026-01-01, 61d
        Extended :ers_94, after srs_94, 61d

    section 9.5 (Unreleased)
        Standard :srs_95, 2026-02-01, 61d
        Extended :ers_95, after srs_95, 61d

    Example Current Date : vert, current, 2026-01-17, 1d
```

In the example shown in the diagram (with `Example Current Date` set to January 17, 2026):

**Major Releases:**
- **Version 9.0 Major** (released September 1, 2025): Currently in <span
  style={{color: "#0f054c",  backgroundColor: "#bec7ff", borderRadius: "10px",
  padding: "3px 10px"}}>Long Term Support</span> phase, with full support
  continuing until May 1, 2026.

**Minor Releases:**
- **Version 9.1 Minor** (released October 1, 2025): Currently in <span
  style={{color: "#0f054c",  backgroundColor: "#2de8b9", borderRadius: "10px",
  padding: "3px 10px"}}>Extended Release Support</span> phase, having completed
  its 2-month standard support period. Extended support continues until February 1,
  2026.
- **Version 9.2 Minor** (released November 1, 2025): Currently in <span
  style={{color: "#0f054c",  backgroundColor: "#2de8b9", borderRadius: "10px",
  padding: "3px 10px"}}>Extended Release Support</span> phase, having completed
  its 2-month standard support period. Extended support continues until March 1,
  2026.
- **Version 9.3 Minor** (released December 1, 2025): Currently in <span
  style={{color: "#fff",  backgroundColor: "#0f054c", borderRadius: "10px",
  padding: "3px 10px"}}>Standard Release Support</span> phase, with full support
  continuing until February 1, 2026, then extended support until April 1, 2026.
- **Version 9.4 Minor** (released January 1, 2026): Currently in <span
  style={{color: "#fff",  backgroundColor: "#0f054c", borderRadius: "10px",
  padding: "3px 10px"}}>Standard Release Support</span> phase, with full support
  continuing until March 1, 2026, then extended support until May 1, 2026.
- **Version 9.5 Minor** (to be released February 1, 2026): Unreleased.

The diagram illustrates the overlapping support windows that provide enterprise
customers with migration flexibility. Major releases have a single Long Term Support (LTS)
model with 8 months of total support coverage,
while Minor releases have a two-tier model (Standard → Extended) with 4 months
of total support coverage.

## Major & Minor Release Process

Each release follows this general schedule:

1. **Feature Development**: Active development phase (Major releases only)
2. **Feature Freeze**: 1 month before release date
3. **Release Candidate**: 1-2 weeks before release date  
4. **Final Release**: On scheduled date

## Security and Patch Updates

- **Security patches**: Released as needed for all supported versions
- **Regular patches**: Bi-weekly review cycle for dependencies
- **Emergency patches**: Released within 24-48 hours for critical security
  issues

## Version Release Lifecycle

Each major and minor version follows a structured release process that includes
pre-releases, the final release, and subsequent patch releases during its
support lifecycle. This section illustrates the complete lifecycle of a single
major version from initial development to end of support.

### Version 9.0.x Series Release Timeline (Example)

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
    todayMarker off
    dateFormat  YYYY-MM-DD
    axisFormat  %b %Y
    tickInterval 3month
    
    section Feature Development Phase
    Feature Development     :done, feat_dev, 2025-04-01, 90d
    _                        :done, feat_blank, 2025-04-01, 0
    
    section Feature Freeze Phase
    Release Candidate 0     :done, rc_0, 2025-07-01, 30d
    Release Candidate 1     :done, rc_1, after rc_0, 15d
    Release Candidate 2     :done, rc_2, after rc_1, 15d
    Final Release           :done, final_release, after rc_2, 7d

    section Long Term Support Phase
    Version 9.0.0         :milestone, 2025-09-02, 0d
    Bugfix Release 9.0.1  :done, patch_1, 2025-10-15, 1d
    Security Release 9.0.2 :crit, sec2, 2025-12-15, 1d
    Security Release 9.0.3 :crit, sec3, 2026-02-01, 1d
    Final LTS Patch 9.0.4 :active, patch_4, 2026-04-30, 1d
```

### Release Details Table (Example)

| Release | Release Date | Type | Purpose & Rationale |
|---------|-------------|------|-------------------|
| **Feature Development** | Apr 1 - Jun 30, 2025 | Development Phase | Active feature development period for Major release 9.0. New features, API enhancements, and architectural improvements. Breaking changes allowed during this phase. |
| **9.0.0-rc.0** | Jul 1, 2025 | Release Candidate | <p>**Feature Freeze Phase**: Initial release candidate for community testing. Major features freeze completed. Focus on stability testing and performance validation. </p><p>Sometimes used by customers depending on the new breaking changes and features for early testing and integration work.</p> |
| **9.0.0-rc.1** | Aug 1, 2025 | Release Candidate | **Feature Freeze Phase**: Second release candidate addressing critical bugs found in rc.0. Database migration optimizations and API refinements. |
| **9.0.0-rc.2** | Aug 15, 2025 | Release Candidate | **Feature Freeze Phase**: Third release candidate for final testing. Documentation finalization and UI/UX polish. Performance benchmarking completed. |
| **9.0.0** | Sep 1, 2025 | **Major Final** | **Official Major release**. All quality gates passed. Production-ready with full documentation and often, a security audit is also completed too. |
| **9.0.1** | Oct 15, 2025 | Bugfix Patch | **Long Term Support Phase**: Address non-critical bugs reported in production: memory leak in vote processing, timezone handling issues, minor UI inconsistencies. |
| **9.0.2** | Dec 15, 2025 | Security Patch | **Long Term Support Phase**: Critical security update addressing privilege escalation vulnerability. Updated cryptographic libraries and enhanced input validation. |
| **9.0.3** | Feb 1, 2026 | Security Patch | **Long Term Support Phase**: Security-only patch addressing newly discovered authentication bypass vulnerability. |
| **9.0.4** | Apr 30, 2026 | Final LTS Patch | **Long Term Support Phase**: Final security patch before end-of-life. Last critical security fix. End of support announced for May 1, 2026. |

### Release Process Timeline

Before diving into a specific example, it's important to understand the
conceptual framework that governs all major version releases. This process
ensures quality, stability, and predictable timing for enterprise customers.

#### Typical Release Process Flow

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
| **Feature Development** | 1-3 months | Active development, new features, breaking changes allowed | No mandatory wait |
| **Feature Freeze to RC.0** | 2-4 weeks | Code stabilization, initial testing | No mandatory wait |
| **Between Release Candidates** | 1-2 weeks | Bug fixes, regression testing | Minimum 1 week |
| **Final RC to Major Release** | **2 weeks** | **Mandatory stabilization period** | **Exactly 2 weeks** |
| **Post-Release Monitoring** | 2-4 weeks | Production stability validation | N/A |

#### Critical Rules

1. **Feature Development Phase**: During this phase, new features are actively
   developed and breaking changes are allowed. This phase typically lasts 1-3
   months depending on the scope of the Major release.

2. **Feature Freeze**: All new features must be code-complete and merged before
   the feature freeze deadline. Only bug fixes and stabilization work are
   allowed after this point.

3. **Mandatory 2-Week Period**: There must be exactly 2 weeks between the final
   release candidate and the Major release. This is non-negotiable and allows
   for:
   - Final security audits
   - Documentation review and finalization
   - Community feedback integration
   - Infrastructure preparation for release

2. **Release Candidate Progression**: Each release candidate must be available
   for at least 1 week before the next RC or final release.

3. **No Direct-to-Production**: All Major releases must go through at least one
   release candidate phase.

4. **Emergency Exception Process**: In case of critical security
   vulnerabilities, the 2-week period may be shortened to 1 week with explicit
   approval from the security team and release management.

