<?php

/*
 * Your custom configuration overrides.
 * This file is loaded last and will override all other settings.
 */

// This enables the SAML IdP and allows IdP-initiated SSO
$config['enable.saml20-idp'] = true;
$config['saml20.idp.allowunsolicited'] = true;

// This is the crucial fix for the "Unable to send artifact" error
$config['store.type'] = 'memcache';
$config['session.cookie.secure'] = false;
$config['session.cookie.samesite'] = 'Lax';
$config['session.cookie.path'] = '/';
