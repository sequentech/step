---
id: demo_mode
title: Demo mode in the Voting Portal
description: How demo mode is triggered, what users see, and how to customize it.
---

Overview
- Demo mode visually marks the Voting Portal when running a non-production demo or preview.
- It shows a tiled DEMO background watermark and a warning dialog to voters.

When demo mode is active
- From Admin Portal preview: entering the Voting Portal via the Publish > Preview flow sets a session flag and enables demo mode automatically.
  - Technical detail: sessionStorage key isDemo is set to true when loading a preview publication event.
- From election configuration: if the ballot style’s public key indicates a demo election (ballot_eml.public_key.is_demo) (which happens when the election has no generated keys), demo mode is active when voters log in.

What users see
- Background watermark: a tiled DEMO image appears across the app background.
  - Image path: /demo-banner.png
  - Container CSS class: watermark-background
- Warning dialog: a dismissible warning appears on the election start screen (not on the election chooser).
  - Dialog CSS class: demo-dialog

Styling and customization
- You can target the following selectors in your CSS/theme to customize styles:
  - .watermark-background for the tiled background (e.g., opacity, blend mode, size)
  - .demo-dialog for the warning dialog (e.g., borders, colors)
- In order to add the custom CSS use Election Event > Data > Ballot Design > Custom CSS, add the custom CSS there and then make a new publication.

Tips
- To test from Admin Portal, use Publish > Preview, then proceed to the Voting Portal; you should see the DEMO background and dialog.
- To test as a voter, ensure the election has no generated keys.
