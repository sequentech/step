---
id: import_format_election_contest_candidate
title: Elecciones, preguntas y candidatos en el archivo del proceso electoral
sidebar_position: 2
description: "Las listas de elecciones, preguntas y candidatos del archivo JSON del proceso electoral: cada campo, los valores permitidos, cómo se refieren los objetos entre sí y un ejemplo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Esta página indica el formato de las listas `elections`, `contests` y `candidates` del archivo JSON
del proceso electoral. Las demás partes del archivo están en [Archivo del proceso electoral](01-election-event.md).
Use esta página para leer una exportación o para comprobar un archivo antes de importarlo.

:::note NOTA: archivo del proceso electoral
Cree el archivo del proceso electoral con **Exportar** en el proceso electoral. Cambie el archivo a
mano solo si es necesario, y solo los campos de esta página. Importe el archivo modificado primero
en un proceso electoral de prueba.
:::

## Cómo se relacionan los objetos {#how-the-objects-are-linked}

El archivo tiene tres listas planas. Los objetos se refieren entre sí mediante IDs:

```text
election_event.id
 └─ elections[].election_event_id      (one election event, many elections)
     elections[].id
      └─ contests[].election_id        (one election, many contests)
          contests[].id
           └─ candidates[].contest_id  (one contest, many candidates)
```

- Cada elección, pregunta y candidato tiene `tenant_id` y `election_event_id`. Use los valores de
  `tenant_id` y `election_event.id` al principio del archivo.
- `contests[].election_id` debe ser el `id` de una elección de `elections`.
- `candidates[].contest_id` debe ser el `id` de una pregunta de `contests`.
- Los vínculos entre áreas y preguntas están en `area_contests`. Consulte
  [Archivo del proceso electoral](01-election-event.md).

### IDs {#ids}

- Cada `id` y cada referencia a un `id` es un UUID versión 4, por ejemplo
  `3f2b8c1e-5d6a-4b7c-9e8f-0a1b2c3d4e5f`. La importación rechaza una elección, una pregunta o un
  candidato cuyo ID no sea un UUID versión 4, por ejemplo `election-1`.
- La importación da IDs nuevos. Sustituye cada UUID del archivo por un UUID versión 4 nuevo. El
  mismo UUID antiguo siempre recibe el mismo UUID nuevo, así que los vínculos siguen siendo
  correctos. Esto también se aplica a los UUID que aparecen en `presentation`, `annotations` y en
  los campos de texto.
- La importación pone el proceso electoral en el inquilino en el que ha iniciado sesión. Sustituye
  el `tenant_id` del archivo por el ID de ese inquilino.

:::caution PRECAUCIÓN: ID de la pregunta de un candidato
Asegúrese de que `candidates[].contest_id` es el `id` de una pregunta del archivo. Si no es un UUID
versión 4, la importación no se detiene: añade el candidato sin pregunta, y el candidato no aparece
en ninguna papeleta.
:::

### Valores que cambia la importación {#values-that-the-import-changes}

Para cada elección, la importación:

- Pone el estado de votación de los canales en línea y de quiosco en `NOT_STARTED`.
- Borra las fechas de inicio, de pausa y de fin de estos canales.
- Pone `statistics` a cero y `initialization_report_generated` en `false`.

La importación mantiene los demás campos tal como están en el archivo.

:::caution PRECAUCIÓN: anotaciones de preguntas, candidatos y elecciones
Asegúrese de que cada valor de `annotations` de una pregunta, un candidato o una elección es texto.
Si un valor es un número, un objeto o una lista, la publicación de la papeleta falla.
:::

### Tipos comunes {#common-types}

| Tipo | Formato |
| --- | --- |
| UUID | Un UUID versión 4 como texto. |
| Fecha y hora | Texto ISO 8601 con zona horaria, por ejemplo `2026-05-01T08:00:00Z`. |
| Mapa i18n | Un objeto. Cada clave es un código de idioma (`en`, `es`, ...). Cada valor es un objeto de campos de texto, por ejemplo `{"en": {"name": "Election 1"}, "es": {"name": "Elección 1"}}`. |
| Anotaciones | Un objeto. **Cada valor debe ser texto.** Para guardar un objeto, guárdelo como texto JSON. |
| JSON | Cualquier valor JSON. |

## Elecciones {#elections}

Cada objeto de `elections` es una elección.

| Campo | Tipo | Obligatorio | Por defecto | Significado |
| --- | --- | --- | --- | --- |
| `id` | UUID | Sí | - | El ID de la elección. |
| `tenant_id` | UUID | Sí | - | El inquilino. |
| `election_event_id` | UUID | Sí | - | El proceso electoral. |
| `name` | Texto | Sí | - | El nombre de la elección. El portal de administración lo copia de `presentation.i18n.en.name`. |
| `alias` | Texto | No | `null` | El ID externo de la elección, por ejemplo el ID en otro sistema. El portal de administración lo muestra como **ID externo**. Después de definirlo, el portal de administración no le permite cambiarlo. |
| `created_at` | Fecha y hora | No | - | No se importa. La importación pone la hora de la importación. |
| `last_updated_at` | Fecha y hora | No | - | No se importa. La importación pone la hora de la importación. |
| `labels` | JSON | No | `null` | Interno. Mantenga el valor de la exportación. |
| `annotations` | Anotaciones | No | `null` | Datos adicionales. Cada valor debe ser texto. |
| `description` | Texto | No | `null` | La descripción. El portal de administración la copia de `presentation.i18n.en.description`. |
| `presentation` | Objeto | No | `null` | La configuración de la elección en el portal de votación: nombres, idiomas, diseño de la papeleta y políticas. Consulte [Archivo de presentación del proceso electoral](09-election-presentation.md). La importación rechaza un valor que no es correcto. |
| `status` | Objeto | No | `null` | El estado de la elección. Consulte [Estado de la elección](#election-status). |
| `eml` | Texto | No | `null` | Interno. Mantenga el valor de la exportación. |
| `num_allowed_revotes` | Entero | No | `null` (= 1) | El número de veces que un votante puede emitir una papeleta en la elección. `0` significa sin límite. Si el campo está vacío, el límite es 1. |
| `is_consolidated_ballot_encoding` | Booleano | No | `null` | Interno. Mantenga el valor de la exportación. |
| `spoil_ballot_option` | Booleano | No | `null` | Interno. Mantenga el valor de la exportación. |
| `is_kiosk` | Booleano | No | `null` | Interno. Mantenga el valor de la exportación. |
| `voting_channels` | Objeto | No | `null` | Los canales de votación de la elección. Consulte [Canales de votación](#voting-channels). |
| `image_document_id` | Texto | No | `null` | El ID del documento de imagen de la elección. En un archivo `.json` sin documentos, la imagen no está en el archivo. |
| `statistics` | Objeto | No | - | La importación lo pone a cero: `{"num_emails_sent": 0, "num_sms_sent": 0}`. |
| `receipts` | Objeto | No | `null` | Los comprobantes que reciben los votantes. Consulte [Comprobantes](#receipts). |
| `permission_label` | Texto | No | `null` | Solo los usuarios del portal de administración con esta etiqueta de permiso pueden ver y cambiar la elección. |
| `keys_ceremony_id` | UUID | No | `null` | La ceremonia de claves de la elección. Úselo solo si el archivo también contiene esta ceremonia de claves. |
| `initialization_report_generated` | Booleano | No | - | La importación lo pone en `false`. |

El nombre, la descripción y el resto de textos de cada idioma están en `presentation.i18n`.

### Estado de la elección {#election-status}

Todos los campos son opcionales. Si falta un campo, recibe el valor por defecto.

| Campo | Tipo | Por defecto | Valores permitidos y significado |
| --- | --- | --- | --- |
| `is_published` | Booleano | `false` | `true` si se publicó una papeleta de la elección. |
| `voting_status` | Texto | `NOT_STARTED` | El estado del canal en línea: `NOT_STARTED`, `OPEN`, `PAUSED`, `CLOSED`. La importación lo pone en el valor inicial. |
| `kiosk_voting_status` | Texto | `NOT_STARTED` | Los mismos valores, para el canal de quiosco. La importación lo pone en el valor inicial. |
| `voting_period_dates` | Objeto | vacío | Las fechas del canal en línea. La importación lo borra. |
| `kiosk_voting_period_dates` | Objeto | vacío | Las fechas del canal de quiosco. La importación lo borra. |
| `init_report` | Texto | `allowed` | `allowed` o `disallowed`. Si es `disallowed`, no puede crear el informe de inicialización. |
| `allow_tally` | Texto | `allowed` | `allowed`, `disallowed` o `requires-voting-period-end`. El ajuste **Permitir Recuento** de la elección. |

Cada objeto de fechas tiene estos campos, todos opcionales, como fecha y hora: `first_started_at`,
`last_started_at`, `first_paused_at`, `last_paused_at`, `first_stopped_at`, `last_stopped_at`.

### Canales de votación {#voting-channels}

| Campo | Tipo | Significado |
| --- | --- | --- |
| `online` | Booleano | Los votantes pueden votar en el portal de votación. |
| `kiosk` | Booleano | Los votantes pueden votar en un quiosco. |

En esta versión solo se usan `online` y `kiosk`. No añada otros canales.

### Comprobantes {#receipts}

Un objeto con las claves `EMAIL`, `SMS` y `DOCUMENT`. Cada valor es un objeto:

| Campo | Tipo | Significado |
| --- | --- | --- |
| `allowed` | Booleano | El votante puede obtener el comprobante por este método. |
| `template` | Texto o `null` | La plantilla que usa el portal de administración para este método, o `null`. |

## Preguntas {#contests}

Cada objeto de `contests` es una pregunta de una elección.

| Campo | Tipo | Obligatorio | Por defecto | Significado |
| --- | --- | --- | --- | --- |
| `id` | UUID | Sí | - | El ID de la pregunta. |
| `tenant_id` | UUID | Sí | - | El inquilino. |
| `election_event_id` | UUID | Sí | - | El proceso electoral. |
| `election_id` | UUID | Sí | - | La elección de la pregunta. |
| `created_at`, `last_updated_at` | Fecha y hora | No | - | No se importan. La importación pone la hora de la importación. |
| `labels` | JSON | No | `null` | Interno. Mantenga el valor de la exportación. |
| `annotations` | Anotaciones | No | `null` | Datos adicionales. Cada valor debe ser texto. |
| `is_acclaimed` | Booleano | No | ninguno | Un indicador guardado. El portal de administración lo muestra como **Is Acclaimed**. El portal de votación no lo usa en esta versión. |
| `is_active` | Booleano | No | `null` | El portal de administración pone `true` en una pregunta nueva. Mantenga el valor de la exportación. |
| `name` | Texto | No | `null` | El nombre de la pregunta. El texto de la presentación es el que ven los votantes. |
| `description` | Texto | No | `null` | La descripción. El portal de administración la copia de `presentation.i18n.en.description`. |
| `presentation` | Objeto | No | vea [Presentación de la pregunta](#contest-presentation) | La configuración de la pregunta en la papeleta. La importación rechaza un valor que no es correcto. |
| `min_votes` | Entero | No | ninguno | El número mínimo de selecciones que debe hacer el votante. El portal de administración pone `0` en una pregunta nueva. |
| `max_votes` | Entero | No | ninguno | El número máximo de selecciones que puede hacer el votante. El portal de administración pone `1` en una pregunta nueva. |
| `winning_candidates_num` | Entero | No | ninguno | El número de ganadores. El portal de administración pone `1` en una pregunta nueva. |
| `voting_type` | Texto | No | `null` | Solo `non-preferential` en el portal de administración. El portal de administración pone `non-preferential` en una pregunta nueva. |
| `counting_algorithm` | Texto | Sí | `plurality-at-large` | El método de recuento. Consulte [Algoritmos de recuento](#counting-algorithms). |
| `is_encrypted` | Booleano | No | `false` | Interno. Mantenga el valor de la exportación. |
| `tally_configuration` | Objeto | No | `null` | La configuración del recuento, como objeto JSON. Mantenga el valor de la exportación. |
| `conditions` | JSON | No | `null` | Interno. Mantenga el valor de la exportación. |
| `image_document_id` | Texto | No | `null` | El ID del documento de imagen de la pregunta. |

:::caution PRECAUCIÓN: algoritmo de recuento de una pregunta
Ponga `counting_algorithm` en `plurality-at-large`. En esta versión, el recuento solo admite este
valor. Si el valor está vacío o es otro, el recuento de la elección falla.
:::

### Algoritmos de recuento {#counting-algorithms}

En esta versión, el único algoritmo de recuento es `plurality-at-large`. El portal de administración
ofrece solo este valor. El recuento no admite otros valores.

### Presentación de la pregunta {#contest-presentation}

Todos los campos son opcionales.

| Campo | Tipo | Por defecto | Valores permitidos y significado |
| --- | --- | --- | --- |
| `i18n` | Mapa i18n | ninguno | El texto de la pregunta en cada idioma. Claves: `name`, `alias`, `description`. Si `alias` está definido, el portal de votación lo muestra en lugar del nombre. |
| `allow_writeins` | Booleano | `false` | `true`: el votante puede escribir un nombre en los candidatos por escrito de la pregunta. |
| `base32_writeins` | Booleano | `true` | `true`: el texto de los candidatos por escrito usa un conjunto reducido de caracteres (Base32). `false`: un conjunto mayor de caracteres. |
| `invalid_vote_policy` | Texto | `allowed` | Lo que hace el portal de votación con una papeleta inválida. Consulte [Políticas de la pregunta](#contest-policies). |
| `under_vote_policy` | Texto | `allowed` | Lo que hace el portal de votación si el votante selecciona menos de `max_votes`. |
| `blank_vote_policy` | Texto | `allowed` | Lo que hace el portal de votación con un voto en blanco. |
| `over_vote_policy` | Texto | `allowed-with-msg-and-alert` | Lo que hace el portal de votación si el votante selecciona más de `max_votes`. El portal de administración escribe `allowed` en una pregunta nueva. |
| `pagination_policy` | Texto | `""` | Un nombre de página. El portal de votación muestra en la misma página las preguntas con el mismo valor, en el orden de la papeleta. Vacío: todas las preguntas sin valor comparten una página. |
| `cumulative_number_of_checkboxes` | Entero | `1` | El número de puntos que el votante puede dar a un candidato. |
| `shuffle_categories` | Booleano | `false` | `true`: el portal de votación muestra las listas (tipos de candidato) en orden aleatorio. `false`: usa el `sort_order` de `types_presentation`. |
| `shuffle_category_list` | Lista de texto | ninguno | Las listas (tipos de candidato) cuyos candidatos el portal de votación muestra en orden aleatorio. |
| `show_points` | Booleano | `false` | `true`: el Verificador de papeletas muestra en su pantalla de confirmación los puntos que el algoritmo de recuento da a cada selección. `false`: no se muestran puntos. |
| `enable_checkable_lists` | Texto | ninguno (el portal de administración escribe `allow-selecting-candidates-and-lists`) | Lo que puede seleccionar el votante cuando la pregunta tiene listas: `allow-selecting-candidates-and-lists`, `allow-selecting-candidates`, `allow-selecting-lists`, `disabled`. |
| `candidates_order` | Texto | `alphabetical` | `alphabetical`, `random`, `custom`. Con `custom`, el orden es el `sort_order` de cada candidato. |
| `candidates_selection_policy` | Texto | `cumulative` | `radio`: una selección nueva elimina la selección anterior. `cumulative`: las selecciones se suman. |
| `candidates_icon_checkbox_policy` | Texto | `square-checkbox` | `square-checkbox` o `round-checkbox`. |
| `max_selections_per_type` | Entero | ninguno | El número máximo de candidatos seleccionados del mismo `type`. |
| `types_presentation` | Objeto | ninguno | La presentación de cada `type` de candidato. Consulte [Presentación de los tipos](#types-presentation). |
| `sort_order` | Entero | ninguno | La posición de la pregunta cuando la elección usa el orden `custom` de preguntas. |
| `columns` | Entero | `1` | El número de columnas de candidatos en la papeleta. |

Los valores por defecto son los que usa el sistema cuando falta el campo. El portal de administración
escribe la mayoría de estos campos cuando guarda una pregunta. Si la pregunta no tiene
`presentation`, no se permiten candidatos por escrito (`allow_writeins` es `false`).

#### Políticas de la pregunta {#contest-policies}

| Campo | Valores permitidos |
| --- | --- |
| `invalid_vote_policy` | `allowed`, `warn`, `warn-invalid-implicit-and-explicit`, `not-allowed` |
| `under_vote_policy` | `allowed`, `warn`, `warn-only-in-review`, `warn-and-alert` |
| `blank_vote_policy` | `allowed`, `warn`, `warn-only-in-review`, `not-allowed` |
| `over_vote_policy` | `allowed`, `allowed-with-msg`, `allowed-with-msg-and-alert`, `not-allowed-with-msg-and-alert`, `not-allowed-with-msg-and-disable` |

#### Presentación de los tipos {#types-presentation}

Un objeto. Cada clave es un `type` de candidato. Cada valor es un objeto:

| Campo | Tipo | Significado |
| --- | --- | --- |
| `name` | Texto | El nombre del tipo. |
| `name_i18n` | Objeto | El nombre en cada idioma: `{"en": "North"}`. |
| `sort_order` | Entero | La posición del tipo. |
| `subtypes_presentation` | Objeto | Cada clave es un `subtype` de candidato. Cada valor tiene `name`, `name_i18n` y `sort_order`. |

## Candidatos {#candidates}

Cada objeto de `candidates` es un candidato. Las opciones explícitas de voto en blanco y de voto
inválido, y los campos de candidatos por escrito, también son candidatos.

| Campo | Tipo | Obligatorio | Por defecto | Significado |
| --- | --- | --- | --- | --- |
| `id` | UUID | Sí | - | El ID del candidato. |
| `tenant_id` | UUID | Sí | - | El inquilino. La importación lo pone en el inquilino de la importación. |
| `election_event_id` | UUID | Sí | - | El proceso electoral. La importación lo pone en el proceso electoral de la importación. |
| `contest_id` | UUID | No | `null` | La pregunta del candidato. Un candidato sin pregunta no aparece en ninguna papeleta. |
| `created_at`, `last_updated_at` | Fecha y hora | No | - | No se importan. La importación pone la hora de la importación. |
| `labels` | JSON | No | `null` | Interno. Mantenga el valor de la exportación. |
| `annotations` | Anotaciones | No | `null` | Datos adicionales. Cada valor debe ser texto. |
| `name` | Texto | No | `null` | El nombre del candidato. El texto de la presentación es el que ven los votantes. |
| `alias` | Texto | No | `null` | El alias del candidato. |
| `description` | Texto | No | `null` | La descripción. |
| `type` | Texto | No | `null` | La lista (categoría) del candidato, como texto libre. El portal de votación muestra juntos los candidatos con el mismo `type`. `types_presentation` de la pregunta usa este valor. |
| `presentation` | Objeto | No | vea [Presentación del candidato](#candidate-presentation) | La configuración del candidato en la papeleta. La importación rechaza un valor que no es correcto. |
| `is_public` | Booleano | No | `null` | Interno. Mantenga el valor de la exportación. |
| `image_document_id` | Texto | No | `null` | El ID del documento de imagen del candidato. |

### Presentación del candidato {#candidate-presentation}

Todos los campos son opcionales.

| Campo | Tipo | Por defecto | Valores permitidos y significado |
| --- | --- | --- | --- |
| `i18n` | Mapa i18n | ninguno | El texto del candidato en cada idioma. Claves: `name`, `alias`, `description`. |
| `is_explicit_invalid` | Booleano | `false` | `true`: esta opción hace que la papeleta sea inválida a propósito. |
| `is_explicit_blank` | Booleano | `false` | `true`: esta opción es un voto en blanco. |
| `is_disabled` | Booleano | `false` | `true`: el portal de votación no muestra el candidato, por ejemplo un candidato retirado. |
| `is_category_list` | Booleano | `false` | `true`: este candidato es el encabezado de la lista con el mismo `type`. `enable_checkable_lists` de la pregunta indica si el votante puede seleccionar toda la lista. |
| `invalid_vote_position` | Texto | ninguno | Para una opción de voto inválido explícito: `top` (arriba) o `bottom` (abajo) de la pregunta. |
| `is_write_in` | Booleano | `false` | `true`: es un campo de candidato por escrito. El votante escribe un nombre. |
| `sort_order` | Entero | ninguno | La posición del candidato cuando la pregunta usa el orden de candidatos `custom`. |
| `urls` | Lista | ninguno | Los enlaces del candidato. Consulte [URL del candidato](#candidate-urls). |
| `subtype` | Texto | ninguno | El subtipo del candidato dentro de su `type`. |

#### URL del candidato {#candidate-urls}

Cada objeto de `urls`:

| Campo | Tipo | Obligatorio | Significado |
| --- | --- | --- | --- |
| `url` | Texto | Sí | La dirección. |
| `kind` | Texto | No | El tipo de enlace. |
| `title` | Texto | No | El título del enlace. |
| `is_image` | Booleano | Sí | `true`: el portal de votación usa esta dirección como imagen del candidato. |

## Ejemplo {#example}

Una elección con una pregunta y dos candidatos. Las demás partes del archivo se omiten. Use sus
propios UUID versión 4.

```json
{
  "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
  "election_event": { "id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d" },
  "elections": [
    {
      "id": "2b3c4d5e-6f70-4b81-9c2d-3e4f5a6b7c8d",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "name": "election-1",
      "description": "Election 1",
      "presentation": {
        "i18n": { "en": { "name": "election-1", "alias": "Election 1" } },
        "language_conf": { "enabled_language_codes": ["en"], "default_language_code": "en" }
      },
      "voting_channels": { "online": true, "kiosk": false },
      "num_allowed_revotes": 1,
      "alias": "election-1"
    }
  ],
  "contests": [
    {
      "id": "3c4d5e6f-7081-4c92-8d3e-4f5a6b7c8d9e",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "election_id": "2b3c4d5e-6f70-4b81-9c2d-3e4f5a6b7c8d",
      "min_votes": 0,
      "max_votes": 1,
      "winning_candidates_num": 1,
      "voting_type": "non-preferential",
      "counting_algorithm": "plurality-at-large",
      "is_encrypted": true,
      "presentation": {
        "i18n": { "en": { "name": "contest-1", "description": "Contest 1" } },
        "candidates_order": "alphabetical",
        "invalid_vote_policy": "allowed",
        "over_vote_policy": "not-allowed-with-msg-and-alert"
      }
    }
  ],
  "candidates": [
    {
      "id": "4d5e6f70-8192-4da3-9e4f-5a6b7c8d9e0f",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "contest_id": "3c4d5e6f-7081-4c92-8d3e-4f5a6b7c8d9e",
      "type": "North",
      "presentation": { "i18n": { "en": { "name": "candidate-1" } } }
    },
    {
      "id": "5e6f7081-92a3-4eb4-8f5a-6b7c8d9e0f1a",
      "tenant_id": "0b1c2d3e-4f50-4a61-8b72-c3d4e5f60718",
      "election_event_id": "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d",
      "contest_id": "3c4d5e6f-7081-4c92-8d3e-4f5a6b7c8d9e",
      "type": "North",
      "presentation": { "i18n": { "en": { "name": "candidate-2" } } }
    }
  ]
}
```

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| La importación falla con `invalid UUID` o `is not v4`. | Asegúrese de que cada `id`, `tenant_id`, `election_event_id`, `election_id` y `keys_ceremony_id` es un UUID versión 4. |
| La importación falla con un error sobre un `presentation`. | Un campo de `presentation` tiene un tipo incorrecto o un valor que no está permitido. Corríjalo con las tablas de esta página. |
| Un candidato no aparece en la papeleta. | Asegúrese de que `contest_id` es el `id` de una pregunta del archivo, y de que `presentation.is_disabled` no es `true`. |
| El recuento de la elección falla. | Ponga `counting_algorithm` de cada pregunta en `plurality-at-large`. |
| La publicación falla con un error sobre anotaciones. | Asegúrese de que cada valor de `annotations` es texto. |
