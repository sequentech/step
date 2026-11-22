---
id: election_management_election_event_localization
title: Localización
description: "Esta sección le permite reemplazar el texto de los portales por idioma. Los reemplazos del proceso electoral pueden afectar al portal de votación, al verificador de papeletas, al portal de resultados o a todos los portales públicos."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Esta sección le permite reemplazar el texto de los portales por idioma. Los reemplazos del proceso electoral pueden afectar al portal de votación, al verificador de papeletas, al portal de resultados o a todos los portales públicos.

### Pasos {#steps}

- **Seleccionar idioma**  
  Elija el idioma para el que se aplicará la localización.

- **Añadir**
  - **Ámbito del portal**: Seleccione el portal que debe recibir el reemplazo. Seleccione **Global** solo cuando la misma clave y el mismo texto deban usarse en todos los portales públicos.
  - **Clave**: Escriba la clave de traducción que quiere personalizar, sin prefijo de portal.
  - **Valor**: Escriba el texto que se mostrará para el elemento seleccionado.

### Acciones {#actions}

- **Editar** un elemento de localización o cambiar su ámbito de portal.
- **Eliminar** un elemento de localización.

### Campos de presentación {#presentation-fields}

Los ámbitos de portal también se aplican a los campos de presentación traducidos, como el nombre, el alias y la descripción de un proceso electoral, una elección, una pregunta o un candidato. Cada aplicación usa automáticamente su propio ámbito. Por ejemplo, el portal de votación lee `votingPortal:description`, mientras que el verificador de papeletas lee `ballotVerifier:description`.

Para el idioma seleccionado, los campos de presentación se resuelven en este orden:

1. El valor del portal actual, por ejemplo `votingPortal:description`.
2. El valor para todos los portales, por ejemplo `global:description`.
3. El valor de presentación sin prefijo, por ejemplo `description`.
4. El campo heredado de la base de datos, si existe, por compatibilidad con procesos electorales antiguos.

Si no hay ningún valor en el idioma activo, se aplica el mismo orden al idioma predeterminado antes de recurrir al campo heredado.

### Reemplazos existentes {#existing-overrides}

Los reemplazos creados antes de que se introdujeran los ámbitos de portal se muestran como **Heredado (portal de votación)**. Siguen afectando solo al portal de votación, exactamente como antes. Editar y guardar un reemplazo heredado lo promueve al ámbito explícito seleccionado en el editor; no se necesita ninguna migración.

A nivel de almacenamiento y de API, los ámbitos explícitos usan un prefijo con dos puntos, por ejemplo `resultsPortal:resultsPortal.pageTitle`. El portal de administración añade este prefijo automáticamente. Los prefijos conocidos son `global`, `votingPortal`, `ballotVerifier`, `resultsPortal` y `adminPortal`. Las claves del proceso electoral sin prefijo siguen siendo solo del portal de votación por compatibilidad. El verificador de papeletas y el portal de resultados ignoran las claves sin prefijo.

Los reemplazos del portal de resultados forman parte de la instantánea de resultados publicada. Vuelva a publicar los resultados después de cambiar un reemplazo para que aparezca en el sitio publicado.
