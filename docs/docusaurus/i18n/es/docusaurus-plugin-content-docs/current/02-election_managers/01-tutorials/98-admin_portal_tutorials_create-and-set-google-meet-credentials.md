---
id: admin_portal_tutorials_create_and_set_google_meet_credentials
title: Crear y configurar las credenciales de Google Meet
description: "Cree las credenciales de Google Meet y configúrelas en el portal de administración, para que la plataforma pueda crear enlaces de reuniones de vídeo."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Descripción general {#overview}

Cree las credenciales de Google Meet y configúrelas en el portal de administración.

**Nota:** Esta configuración debe hacerse desde la consola de Google Cloud.

## Pasos de configuración {#configuration-steps}

## Configuración de Google Cloud Console {#google-cloud-console-setup}

1. Vaya a [Google Cloud Console](https://console.cloud.google.com/)
2. Cree un proyecto nuevo o seleccione uno existente. Debe tener acceso de superadministrador para configurar después la delegación de dominio en toda la organización.
3. Habilite la API de Google Calendar.
4. Menú de navegación > API y servicios > Credenciales.
5. Cree una cuenta de servicio.
6. Editar > Claves > Añadir clave. Después, descargue el archivo JSON.
7. Añada los dominios autorizados para su aplicación.
8. Pegue el archivo JSON con la clave de la cuenta de servicio en Configuración > INTEGRACIONES > Clave de Cuenta de Servicio de Google Calendar.

## Configurar la delegación de dominio en toda la organización para una cuenta de servicio {#set-up-domain-wide-delegation-for-a-service-account}

1. Siga los [pasos de este enlace](https://developers.google.com/workspace/guides/create-credentials#optional_set_up_domain-wide_delegation_for_a_service_account)
2. Añada los ámbitos para el ID de cliente autorizado: https://www.googleapis.com/auth/calendar, https://www.googleapis.com/auth/calendar.events
3. Pegue la dirección de correo electrónico autorizada en Configuración > INTEGRACIONES > Correo de Autenticación de Google Calendar. **Nota:** Este correo aparecerá como organizador de la reunión.
