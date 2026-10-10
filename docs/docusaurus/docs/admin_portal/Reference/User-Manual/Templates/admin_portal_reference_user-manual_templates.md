---
id: admin_portal_reference_user_manual_templates
title: Admin Portal Reference User Manual Templates
---



This is a placeholder page for the section: Templates.

Content will be added here soon.

## Images, Styles and Scripts in PDF Documents

When a template is rendered to PDF, the document can load:

- files in the platform's public bucket, such as the public assets and documents uploaded as public;
- the assets bundled with the document renderer, under `/assets`;
- inline `data:` and `blob:` URLs.

Other addresses are not loaded, and frames such as `<iframe>` do not load any address. To use an
image, font or stylesheet hosted elsewhere, upload it as a public document or embed it in the
template as a `data:` URL.

Scripts in a template run, but they can only connect to the platform's public bucket. Web workers,
forms and pop-up windows are not available. Documents larger than 48 MiB are rendered with
JavaScript disabled.
