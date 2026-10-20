---
id: languages
title: Idiomas
description: "En el portal de votación, el sistema determina qué idioma mostrar a los votantes según un orden definido."
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
## Determinación del idioma en el portal de votación {#language-determination-in-the-voting-portal}

En el portal de votación, el sistema determina qué idioma mostrar a los votantes según un orden definido. Respeta las preferencias del usuario y permite a los administradores aplicar políticas de idioma cuando es necesario.

### Prioridad para determinar el idioma {#language-determination-priority}

El idioma se determina en el siguiente orden de precedencia (de mayor a menor):

1. **Parámetro de búsqueda de la URL (`lang`)**: prioridad más alta.
2. **Idioma seleccionado por el usuario en el flujo de inicio de sesión**: se guarda en una cookie del navegador.
3. **Política de detección de idioma**: se configura en la pestaña **Datos** del proceso electoral.
4. **Configuración del navegador**: preferencia de idioma del navegador (prioridad más baja).

### Cómo funciona cada nivel {#how-each-level-works}

#### 1. Parámetro de búsqueda de la URL (`lang`) {#1-url-search-parameter-lang}

Cuando un votante accede al portal de votación con el parámetro de consulta `lang`, este tiene prioridad absoluta sobre el resto de ajustes. Así puede dirigir a los votantes a una versión en un idioma concreto mediante enlaces.

**Ejemplo:**
```
https://myelection.sequent.vote/?lang=es
```

El parámetro `lang` se comprueba durante la inicialización de i18n, de modo que el idioma se aplica antes de evaluar cualquier política.

#### 2. Idioma seleccionado por el usuario {#2-user-selected-locale}

Cuando un votante selecciona un idioma en el portal de votación, su elección se guarda en una cookie del navegador. Esta cookie se conserva durante la sesión (hasta que se cierra el navegador). En las visitas siguientes dentro de la misma sesión, se restaura la preferencia de idioma guardada.

Esto permite a los votantes:
- Seleccionar su idioma preferido una sola vez.
- Anular la política de detección de idioma para su sesión individual.

#### 3. Política de detección de idioma {#3-language-detection-policy}

La política de detección de idioma se configura a nivel de proceso electoral y determina cómo el sistema selecciona un idioma cuando no existe ningún parámetro en la URL ni una cookie de preferencia del usuario.

**Políticas disponibles:**

- **`BROWSER_DETECT`** (predeterminada): el portal de votación detecta automáticamente el idioma del navegador del votante y muestra el contenido en el idioma disponible más cercano.
- **`FORCE_DEFAULT`**: todos los votantes ven el idioma predeterminado especificado en la configuración de la elección, sin importar el idioma de su navegador.

**Cuándo se aplica esta política:**
- Solo cuando no hay ningún parámetro `lang` en la URL.
- Solo cuando el usuario no ha cambiado el idioma manualmente.

### Escenarios de ejemplo {#example-scenarios}

**Escenario 1: elección multilingüe con detección por navegador**
- No hay ninguna política de idioma configurada.
- Un votante con el navegador en español ve el portal en español de inmediato.
- Un votante con el navegador en inglés ve el portal en inglés de inmediato.
- Ambos votantes pueden seleccionar manualmente otro idioma, y su elección se guarda.

**Escenario 2: elección bilingüe con idioma predeterminado forzado**
- La política de idioma está configurada como `FORCE_DEFAULT` con `default_language_code: es`.
- Todos los votantes ven el portal en español, sin importar la configuración de su navegador.
- Los votantes pueden seguir eligiendo otro idioma manualmente desde la interfaz.

**Escenario 3: elección con preferencia de usuario persistente**
- No hay ninguna política de idioma configurada.
- Un votante visita el portal en español y selecciona inglés manualmente.
- La preferencia de inglés se guarda en una cookie.
- El parámetro de URL `lang` sigue anulando esta preferencia si se proporciona.
