---
id: accessibility
title: Voting Portal accessibility
description: How the Voting Portal satisfies WCAG 2.0 SC 1.3.1 and 4.1.2 (Level A), and the conventions to follow when changing voter-facing UI.
---

## Conformance target

The Voting Portal targets **WCAG 2.0 Level A** for the voter journey, and this page
documents the two success criteria that have been reviewed end to end:

- **SC 1.3.1 Info and Relationships** — information, structure and relationships conveyed
  through presentation must also be programmatically determinable.
- **SC 4.1.2 Name, Role, Value** — every user interface component must expose a name and a
  role, its state must be programmatically determinable, and changes must be announced.

The reviewed journey is: login → election chooser → start → ballot → review → confirmation,
plus the audit and ballot-locator screens and the support materials screen.

Most of the voter-facing widgets live in the shared `@sequentech/ui-essentials` package, so
the fixes described below are in both `voting-portal` and `ui-essentials`. Components that
only the Admin Portal, Results Portal or Ballot Verifier render (`Tree`,
`ResultsSelectorTabs`, `PreferentialCandidateResults`, `CustomDropFile`, `CountdownBar`,
`PlaintextVoteContest`) were out of scope for this review. Shared components that those
portals also render take their new semantics via opt-in props — `CandidatesList`'s
`titleComponent` and `BreadCrumbSteps`' `ariaLabel` — so the out-of-scope portals keep their
existing markup until they are audited too.

## Conventions to follow when changing voter-facing UI

### Name every control

An icon carries no text, so every icon-only control needs an explicit name.
`IconButton` takes dedicated `ariaLabel` and `ariaLabelledby` props which it puts on the
`<button>`; every other prop is spread onto the icon, where an accessible name would not
reach the button. It still falls back to the placeholder string `"icon button"` so that
call sites in the other portals — which have not been given names yet — are not left with no
name at all. That placeholder is not an acceptable name: always pass `ariaLabel` or
`ariaLabelledby`, and the fallback should be deleted once the other portals are labelled.

Prefer pointing at the visible text with `aria-labelledby` over duplicating it in an
`aria-label`, so the accessible name cannot drift from what is on screen. `Candidate` does
this: the checkbox, the rank select, the write-in field and the "more information" link are
all labelled by the rendered candidate title, composed with a visually hidden qualifier such
as "Preference" or "Write-in candidate name".

Use the shared `VisuallyHidden` component from `ui-essentials` for text that assistive
technology should read but that should not be painted. It wraps MUI's `visuallyHidden`
style, and is the right tool instead of `display: none` — content hidden with `display: none`
is removed from the accessibility tree entirely.

### Keep structure in the markup

- **Headings** — set the level with MUI's `component` prop and the size with `variant`, so
  the visual and programmatic hierarchies can differ without either being wrong. Each voter
  screen has one `<h1>`; contest titles are `<h2>`; candidate list (slate) titles are `<h3>`;
  candidate subtype groups are `<h4>`. One known exception: the ballot locator's logs tab
  unmounts the panel that owns its `<h1>`, so that tab currently starts at `<h2>`.
- **Lists** — repeated items must be `<li>` inside a `<ul>`/`<ol>`. Because the ballot lists
  use `list-style: none`, they also carry an explicit `role="list"`; Safari and VoiceOver
  otherwise drop list semantics from unstyled lists.
- **Grouping** — a contest's options are wrapped in a `<fieldset>` whose visually hidden
  `<legend>` carries both the contest name and how many choices the voter may make, so the
  question and the limit are announced with each option.
- **Injected HTML** — admin-authored HTML is rendered with `stringToHtml`, which sanitises
  then parses. Render it inside `Typography component="div"`, never a default `Typography`:
  the default renders a `<p>`, and a block element inside a `<p>` is auto-closed by the
  parser, silently destroying the structure.

### Announce what changes

Dynamic content must be in a live region or it is invisible to a screen reader user.
`WarnBox` — the ballot's entire validation surface — takes an `EWarnBoxAnnouncement`:
`POLITE` (`role="status"`) is the default, `ASSERTIVE` (`role="alert"`) is reserved for a
single blocking message, and `SILENT` opts out for content already announced some other way
(the write-in length error uses it, because the write-in field points `aria-describedby` at
it). Polite is the default deliberately: a screen renders one message list per contest, and
assertive regions interrupt each other, so all but the last would be lost. Use an enum rather
than a boolean for this kind of policy so it can grow.

A live region must be **mounted before** its text appears — a region inserted at the same
moment as its content is not reliably announced. Keep the region rendered and swap its text,
as the ballot-locator result and the print status do.

Messages that explain why a specific control is rejected must also be linked to it with
`aria-describedby`. The write-in length error does this via the exported `writeInErrorId`
helper.

## Known deviations

Two things were deliberately left as they are. Both are recorded here so a future audit does
not treat them as oversights.

### Single-choice contests use checkboxes, not radio buttons

When a contest is configured with `ECandidatesSelectionPolicy.RADIO` and `max_votes === 1`,
the options behave as a radio group — selecting one clears the others — but they are rendered
as `<input type="checkbox">`, and the `ECandidatesIconCheckboxPolicy.ROUND_CHECKBOX`
presentation policy draws them with radio-shaped icons. A screen reader therefore announces
"checkbox" for a control that looks and behaves like a radio button.

This is intentional. The icon shape is a client-configurable presentation policy rather than
a semantic choice, and native radio buttons cannot be deselected once chosen — switching to
them would remove the voter's ability to clear a selection, which matters for contests where
abstaining is meaningful. The grouping and labelling around these controls has been fixed, so
the contest name and selection limit are announced with each option.

### Whole-row and whole-card click targets are mouse-only

The candidate row (`Candidate`'s `<li>`), the candidate list container and the election card
in `SelectElection` all respond to a click anywhere on them. These are conveniences layered
on top of real controls: each row contains a real checkbox, and each election card contains a
real button, so name, role and value are all correctly exposed and SC 4.1.2 is satisfied.
Full keyboard parity for the surrounding surface is a separate criterion (SC 2.1.1) and was
not in scope.

## Related work outside this scope

The following are known gaps against other success criteria, not covered by this review:

- `public/index.html` hardcodes `lang="en"` and does not follow the language switcher
  (SC 3.1.1 Language of Page).
- The candidate list collapse toggle's `aria-label` does not contain its visible
  "Show/Hide candidates" text (SC 2.5.3 Label in Name).
- Per-tenant CSS from `election_event_presentation.css` is injected verbatim, so a
  deployment can override focus indicators and the visually-hidden utility styles.

## Verifying changes

Automated checks:

```bash
cd packages
yarn lint
yarn prettify:fix:ui-essentials && yarn build:ui-essentials
yarn build:voting-portal
reuse lint
```

Rebuilding `ui-essentials` is required — the Voting Portal consumes its built output, so
component changes do not appear otherwise.

Manual checks, walking the full voter journey with a screen reader and the
[IBM Equal Access toolkit](https://www.ibm.com/able/toolkit/tools/#develop):

1. Every checkbox announces the candidate name; every rank select announces which candidate
   it ranks; the write-in field announces a label; the slate checkbox announces the slate.
2. Each option is announced within its contest, including how many choices are allowed.
3. Over-voting is announced as it happens, and the write-in-too-long message is reachable as
   the field's description.
4. Heading order runs h1 → h2 → h3 with exactly one h1 per screen, and the stepper announces
   the current step.
5. Every dialog announces its title when it opens.
6. A contest description containing a list and a table with header cells keeps both intact.

Because the ballot renders very differently depending on configuration, cover: single versus
multi-select contests; both `ECandidatesIconCheckboxPolicy` values; preferential and plain
counting; write-ins; slates with `ECollapsibleLists` enabled and collapsed; explicit blank
and explicit invalid options; each `EOverVotePolicy` including
`NOT_ALLOWED_WITH_MSG_AND_DISABLE`; paginated multi-page ballots; and `skip_election_list`,
which changes the stepper's step count.
