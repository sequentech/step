---
id: admin_portal_reference_user_manual_settings
title: Configuración
description: "La página Configuración contiene la configuración del inquilino. La configuración se aplica a todos los procesos electorales del inquilino. El administrador del inquilino la establece una vez, antes del primer proceso electoral."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La página **Configuración** contiene la configuración del inquilino. La configuración se aplica
a todos los procesos electorales del inquilino. El administrador del inquilino la establece una
vez, antes del primer proceso electoral. Consulte [Configurar el inquilino](../../../procedures/01-tenant.md).

## Abrir la página {#open-the-page}

En el menú de la izquierda, haga clic en **Configuración**. La página tiene el encabezado
**Configuración** y "Ajustes generales".

El elemento de menú **Configuración** necesita el permiso **Ver configuración**. La página
también necesita el permiso **Editar Inquilino**. Sin ambos, la página muestra "No tienes
permiso para acceder a la configuración.". En la versión 9.0 no se usan los permisos
**Ver configuración de ...** de cada pestaña: se muestran todas las pestañas.

## Pestañas {#tabs}

| Pestaña | Uso | Página de referencia |
| --- | --- | --- |
| **TIPOS DE ELECCIÓN** | Una lista de tipos de elección. La versión 9.0 no la usa en las elecciones. | [Tipos de elección (obsoleto)](election-types-deprecated/admin_portal_reference_user-manual_settings_election-types-deprecated.md) |
| **CANALES DE VOTACIÓN** | Los canales de votación del inquilino. En la interfaz en inglés de la versión 9.0, el nombre de la pestaña tiene un error ortográfico (VOTING CHANELS). | [Canales de votación](Voting-Channels/admin_portal_reference_user-manual_settings_voting-channels.md) |
| **PLANTILLAS** | Muestra si están disponibles el correo electrónico y los mensajes de texto. Solo lectura. | [Plantillas (obsoleto)](templates-deprecated/admin_portal_reference_user-manual_settings_templates-deprecated.md) |
| **IDIOMAS** | Los idiomas que pueden usar los procesos electorales. | [Idiomas](Languages/admin_portal_reference_user-manual_settings_languages.md) |
| **LOCALIZACIÓN** | Cambios en los textos de los portales para todos los procesos electorales. | [Localización](Localization/admin_portal_reference_user-manual_settings_localization.md) |
| **PERSONILIZACIÓN DE APARIENCIA** | El logotipo, el estilo y los enlaces de ayuda del inquilino. | [Personalización de apariencia](look--feel/admin_portal_reference_user-manual_settings_look--feel.md) |
| **AUTORIDADES** | Los fideicomisarios del inquilino. | [Fideicomisarios](Trustees/admin_portal_reference_user-manual_settings_trustees.md) |
| **Countries** | Los países desde los que se bloquean la votación y la inscripción. | [Países](countries-blocked-countries/admin_portal_reference_user-manual_settings_countries-blocked-countries.md) |
| **COPIA DE SEGURIDAD / RESTAURAR** | La copia de seguridad y la restauración de la configuración del inquilino. | [Copia de seguridad / Restaurar](backup--restore/admin_portal_reference_user-manual_settings_backup--restore.md) |

La versión 9.0 no tiene pestaña **Integrations** (integraciones) ni selector de idioma
predeterminado.
