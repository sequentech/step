---
id: settings_localization
title: Localización
description: "Las anulaciones de localización del inquilino personalizan el texto del portal de administración para cada idioma habilitado. El editor ofrece Portal de Administración para una anulación específica del portal y Global para anulaciones de todo el código fuente."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Las anulaciones de localización del inquilino personalizan el texto del portal de administración para cada idioma habilitado. El editor ofrece **Portal de Administración** para una anulación específica del portal y **Global** para anulaciones de todo el código fuente. Como la localización del inquilino solo la usa el portal de administración, ambas opciones llegan actualmente a ese portal. **Global** no se aplica a los portales públicos de los procesos electorales. Escriba la clave de traducción sin prefijo y, después, introduzca su nuevo valor.

Las anulaciones existentes sin prefijo se muestran como **Legacy (Portal de Administración)** y siguen afectando solo al portal de administración. Si edita y guarda una de ellas, pasa al ámbito explícito que seleccione en el editor. No se requiere ninguna migración de datos.

Las claves con ámbito explícito se guardan con un prefijo de dos puntos, como `adminPortal:header.title`. El portal de administración compone la clave guardada automáticamente, así que no escriba el prefijo en el campo de clave.
