---
id: import_format_election_presentation
title: Archivo de presentación de la elección
sidebar_position: 10
description: "El archivo JSON que puede subir en la pestaña Datos de una elección para cambiar su configuración de presentación: cómo funciona la carga, cada campo, los valores permitidos y ejemplos."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La presentación de una elección es un objeto JSON. Contiene los nombres de la elección en cada idioma y la configuración de la elección en el portal de votación: el diseño de la papeleta, las políticas y el período de gracia. La mayoría de los ajustes tienen un campo en la pestaña **Datos** de la elección. Algunos no tienen campo, por ejemplo `voting_period_end` y `css`. Para configurarlos, suba un archivo de presentación de la elección. Esta tarea la realiza un administrador electoral.

## Cómo cambia la presentación al subir el archivo {#how-the-upload-changes-the-presentation}

El portal de administración combina el archivo con la presentación de la elección cuando hace clic en **Guardar**:

- Cada clave de primer nivel del archivo reemplaza la misma clave de la presentación. El portal de administración no combina el contenido de la clave. Por ejemplo, una clave `i18n` del archivo reemplaza los nombres, los alias y las descripciones en **todos** los idiomas.
- Las claves que no están en el archivo no cambian.
- El valor del archivo prevalece sobre el valor del mismo campo en la pantalla.
- El portal de administración no usa `language_conf` del archivo. Siempre escribe `language_conf` a partir de la sección **Idioma** de la pantalla.
- Si sube un segundo archivo antes de hacer clic en **Guardar**, el portal de administración usa solo el segundo archivo.

El portal de administración no comprueba el archivo. Guarda cada clave y cada valor tal como aparecen en el archivo.

:::danger ADVERTENCIA: archivo de presentación de la elección
Use solo las claves y los valores de esta página. Antes de guardar, compruebe que cada valor tiene el tipo correcto y es un valor permitido. Si un valor no es correcto, el sistema no puede leer la presentación de la elección. Entonces ningún votante puede emitir una papeleta en la elección, no puede cerrar el período de votación y falla la publicación de la papeleta.
:::

## Antes de empezar {#before-you-start}

- Tiene el rol de administrador electoral y puede editar la elección.
- Tiene un archivo `.json`. El nivel superior del archivo es un objeto.
- Escriba los caracteres que no son ASCII (por ejemplo `é` o `ñ`) como secuencias de escape `\u`: `é`, `ñ`. La carga lee el archivo byte a byte, así que otros caracteres se muestran incorrectamente.
- Haga el cambio primero en una elección de prueba.

## Subir el archivo {#upload-the-file}

1. Abra la elección en el árbol de procesos electorales.
2. Haga clic en la pestaña **Datos**.
3. Haga clic en **Configuración Avanzada** para abrir la sección.
4. Arrastre el archivo `.json` a la zona de carga, o haga clic en la zona de carga y seleccione el archivo.
5. Haga clic en **Guardar**.

**Resultado esperado:** el portal de administración muestra que la elección se ha guardado. Vuelva a abrir **Configuración Avanzada**. **Previsualizar** muestra la presentación con las claves del archivo.

:::note NOTA: presentación de la elección en la papeleta publicada
La papeleta publicada contiene una copia de la presentación de la elección. Después de cambiar la presentación, publique la papeleta de nuevo para que los votantes vean el cambio.
:::

## Campos {#fields}

Todos los campos son opcionales. El valor predeterminado es el valor que usa el sistema cuando falta el campo.

| Campo | Tipo | Predeterminado | Valores permitidos y significado |
| --- | --- | --- | --- |
| `i18n` | Objeto | ninguno | El texto de la elección en cada idioma. Consulte [i18n](#i18n). |
| `language_conf` | Objeto | ninguno | Los idiomas de la elección. Consulte [Configuración del idioma](#language-configuration). La carga no lo cambia. |
| `contests_order` | Texto | `alphabetical` | El orden de las preguntas en la papeleta. `alphabetical`: por alias, o por nombre si no hay alias. `random`: un orden distinto para cada votante. `custom`: por el `sort_order` de cada pregunta. |
| `audit_button_cfg` | Texto | `show` | El botón **Auditar papeleta**. `show`: se muestra al votante. `not-show`: no se muestra. `show-in-help`: solo en la ayuda. |
| `sort_order` | Entero | ninguno | La posición de la elección en la lista de elecciones, cuando el proceso electoral usa el orden `custom` de elecciones. |
| `cast_vote_confirm` | Booleano | `false` | `true`: el votante debe confirmar en un cuadro de diálogo antes de emitir la papeleta. |
| `cast_vote_gold_level` | Texto | `no-gold-level` | `gold-level`: el votante debe iniciar sesión de nuevo antes de emitir la papeleta. `no-gold-level`: no hay un nuevo inicio de sesión. |
| `start_screen_title_policy` | Texto | `election` | El título y la descripción de la pantalla de inicio. `election`: los de la elección. `election-event`: los del proceso electoral. |
| `grace_period_policy` | Texto | `no-grace-period` | `no-grace-period`: no se puede votar después del fin programado de la votación. `grace-period-without-alert`: un votante que inició sesión antes del fin programado puede emitir una papeleta durante `grace_period_secs` segundos después. Solo para el canal en línea, y no cuando la votación está pausada. |
| `grace_period_secs` | Entero | `0` | La duración del período de gracia en segundos. No puede ser negativa. |
| `voting_period_end` | Texto | `allowed` | `allowed`: puede cerrar el período de votación. `disallowed`: el sistema impide cerrarlo. |
| `initialization_report_policy` | Texto | `not-required` | `required`: no puede abrir la votación antes de generar el informe de inicialización. `not-required`: sin condición. |
| `security_confirmation_policy` | Texto | `none` | `mandatory`: el votante debe marcar una casilla con una declaración antes de que empiece la votación. El texto está en `i18n.<language>.security_confirmation_html`. `none`: sin casilla. |
| `consolidated_report_policy` | Texto | `do-not-generate` | `generate`: el recuento genera resultados consolidados de todas las áreas. `do-not-generate`: sin resultados consolidados. |
| `decline_to_vote_policy` | Texto | `disabled` | `enabled`: el votante puede abstenerse de votar en toda la elección. `disabled`: la opción no se muestra. Consulte [Abstención de voto](../07-decline-to-vote.md). |
| `blank_ballots_policy` | Texto | `disabled` | `enabled`: una papeleta que está en blanco en todas las preguntas cuenta como papeleta en blanco de la elección. `disabled`: no se cuenta. Consulte [Papeletas en blanco](../09-blank-ballots.md). |
| `voting_screen_back_policy` | Texto | `election-selection-screen` | Adónde va el botón **Atrás** de la pantalla de votación. `election-selection-screen`: la lista de elecciones. `start-screen`: la pantalla de inicio de la elección. |
| `css` | Texto | ninguno | CSS personalizado para la elección en el sitio de resultados. El portal de votación usa el CSS del proceso electoral. |
| `custom_filters` | Lista | ninguno | Los filtros personalizados que el portal de administración edita en **Configuración Avanzada**. La lista **Usuarios** lee los filtros del proceso electoral, así que estos filtros no cambian esa lista. |

`decline_to_vote_policy` y `blank_ballots_policy` solo tienen efecto cuando el proceso electoral usa la política de cifrado **Varios concursos**.

### Campos que esta versión no usa {#fields-that-this-version-does-not-use}

La presentación también puede contener estos campos. El sistema los lee sin error, pero en esta versión no los usa. No los añada.

| Campo | Tipo | Valores permitidos |
| --- | --- | --- |
| `dates` | Objeto | `start_date` y `end_date`, como texto de fecha y hora. Configure las fechas de la votación con eventos programados. |
| `is_grace_priod` | Booleano | `true`, `false`. Use `grace_period_policy`. |
| `init_report` | Texto | `allowed`, `disallowed` |
| `manual_start_voting_period` | Texto | `allowed`, `only-when-initialization-report-has-been-performed` |
| `tally` | Texto | `always-allow`, `allow-when-voting-period-ends` |

### i18n

Un objeto. Cada clave es un código de idioma, por ejemplo `en` o `es`. Cada valor es un objeto con estos campos de texto:

| Campo | Significado |
| --- | --- |
| `name` | El nombre de la elección. |
| `alias` | Un nombre corto. Si está configurado, el portal de votación lo muestra en lugar del nombre. |
| `description` | La descripción de la elección. |
| `security_confirmation_html` | La declaración que el votante debe aceptar cuando `security_confirmation_policy` es `mandatory`. Se permite HTML. |

### Configuración del idioma {#language-configuration}

| Campo | Tipo | Significado |
| --- | --- | --- |
| `enabled_language_codes` | Lista de textos | Los idiomas de la elección, por ejemplo `["en", "es"]`. |
| `default_language_code` | Texto | El idioma que se usa cuando el texto no está en el idioma del votante. Si no está configurado, se usa el idioma predeterminado del proceso electoral. |
| `language_detection_policy` | Texto | `browser-detect` o `force-default`. No se usa a nivel de elección. El portal de votación lee este valor del proceso electoral. El portal de administración lo elimina de la elección cuando usted guarda la elección. |

La carga no cambia `language_conf`. Configure los idiomas en la sección **Idioma** de la pestaña **Datos**.

## Ejemplos {#examples}

### Cambiar un ajuste {#change-one-setting}

Este archivo permite cerrar el período de votación. No cambia otros ajustes.

```json
{
  "voting_period_end": "allowed"
}
```

### Ejemplo completo {#full-example}

Este archivo configura cada campo que la carga puede cambiar. La clave `i18n` reemplaza el texto en todos los idiomas, así que contiene todos los idiomas de la elección.

```json
{
  "i18n": {
    "en": {
      "name": "election-1",
      "alias": "Election 1",
      "description": "Election of the North area",
      "security_confirmation_html": "<p>I confirm that I can vote in this election.</p>"
    },
    "es": {
      "name": "election-1",
      "alias": "Elección 1",
      "description": "Elección del área North",
      "security_confirmation_html": "<p>Confirmo que puedo votar en esta elección.</p>"
    }
  },
  "contests_order": "custom",
  "audit_button_cfg": "show-in-help",
  "sort_order": 1,
  "cast_vote_confirm": true,
  "cast_vote_gold_level": "no-gold-level",
  "start_screen_title_policy": "election",
  "grace_period_policy": "grace-period-without-alert",
  "grace_period_secs": 300,
  "voting_period_end": "allowed",
  "initialization_report_policy": "required",
  "security_confirmation_policy": "mandatory",
  "consolidated_report_policy": "generate",
  "decline_to_vote_policy": "disabled",
  "blank_ballots_policy": "disabled",
  "voting_screen_back_policy": "election-selection-screen",
  "css": "h1 { font-weight: bold; }"
}
```

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Después de guardar, los votantes no pueden emitir una papeleta, o la publicación falla con `Error parsing election presentation`. | Un valor de la presentación no es correcto. Suba un archivo que establezca la clave incorrecta con un valor correcto, o abra **Previsualizar** para encontrarla. Después, haga clic en **Guardar**. |
| No puede cerrar el período de votación. | Asegúrese de que `voting_period_end` es `allowed` y de que todos los demás valores son correctos. |
| El texto muestra caracteres incorrectos. | Escriba los caracteres que no son ASCII como secuencias `\u` y suba el archivo de nuevo. |
| Los idiomas no cambiaron. | La carga no cambia `language_conf`. Use la sección **Idioma**. |
| Los nombres en otros idiomas desaparecieron. | La clave `i18n` del archivo reemplazó todos los idiomas. Suba una clave `i18n` que contenga todos los idiomas. |
| Los votantes no ven el cambio. | Publique la papeleta de nuevo. |
