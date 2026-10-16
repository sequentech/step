---
id: voting_portal_datetime_format
title: Formato de fecha y hora del portal de votación
sidebar_position: 8
description: "El formato de fecha y hora del portal de votación controla cómo se muestran las fechas y las horas a los votantes en el portal de votación de un proceso electoral concreto. Se configura de dos formas complementarias:"
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El **formato de fecha y hora del portal de votación** controla cómo se muestran las fechas y las horas a los votantes en el portal de votación de un proceso electoral concreto. Se configura de dos formas complementarias:

- Un **formato para todo el evento**, elegido en el portal de administración: un preajuste de una lista controlada o un **patrón de tokens personalizado**.
- Una **anulación por idioma**, opcional, escrita como una cadena de traducción en la pestaña Localización.

Ambos se guardan en el proceso electoral y los resuelve una única función auxiliar compartida, de modo que todas las fechas visibles para el votante se muestran de forma coherente.

---

## Preajustes {#presets}

El preajuste se configura en **Proceso electoral → Datos → Configuración Avanzada**, en el selector **Formato de fecha y hora del portal de votación**. Se aplica **a todo el evento** (a todos los votantes, sin importar el idioma, salvo que se anule, como se explica más abajo).

El valor predeterminado es **Legacy GB 24h**, que conserva el comportamiento que existía antes de esta función, de modo que los procesos electorales existentes no cambian de aspecto.

Están disponibles los siguientes preajustes (los ejemplos muestran el instante **9 de marzo de 2026, 07:05** en la hora local del votante):

| Preajuste | Valor interno | Ejemplo de salida |
|---|---|---|
| **Legacy GB 24h** (predeterminado) | `legacy-gb-24h` | `09/03/2026, 07:05` |
| **ISO Local** | `iso-local` | `2026-03-09 07:05` |
| **US 12h** | `us-12h` | `03/09/2026, 7:05 AM` |
| **Locale Medium** | `locale-medium` | `9 Mar 2026, 07:05` (en el idioma del votante) |
| **Date Only** | `date-only` | fecha en el idioma del votante, sin componente de hora |

`LEGACY_GB_24H` es el valor predeterminado y reproduce exactamente el formato anterior de la lista de papeletas. `LOCALE_MEDIUM` y `DATE_ONLY` se muestran en el idioma activo del votante; los demás preajustes están fijados a una configuración regional para que la salida sea estable.

---

## Formato personalizado para todo el evento {#custom-event-wide-format}

El mismo selector también ofrece **Formato personalizado**. Al seleccionarlo, aparece el campo **Formato de fecha y hora personalizado**, donde un patrón de tokens (consulte la referencia de tokens más abajo) se aplica **a todo el evento**, igual que un preajuste.

El patrón se valida al guardar la pestaña Datos: un patrón no válido bloquea el guardado y muestra el error debajo del campo, de modo que nunca llega a guardarse. Se almacena dentro del mismo campo `voting_portal_datetime_format` que los preajustes, con el formato `{"custom": "<pattern>"}`.

---

## Anulación por idioma {#per-language-override}

El preajuste puede anularse **por idioma** con el flujo de trabajo de localización existente. Es útil cuando un idioma concreto necesita una distribución de campos distinta de la que ofrece el preajuste para todo el evento.

Configúrelo en **Proceso electoral → Localización**: elija el idioma y añada la clave `votingPortalDateTimeFormat` con un patrón de tokens como valor. La anulación se aplica **solo a ese idioma**; los idiomas sin anulación usan el preajuste del evento.

El valor se interpreta como un **patrón de tokens**. Los tokens se muestran en la hora local del votante; cualquier otro carácter (separadores, espacios, literales) se mantiene sin cambios.

| Token | Significado | Ejemplo (para 2026-03-09 07:05) |
|---|---|---|
| `yyyy` | Año de 4 dígitos | `2026` |
| `MM` | Mes de 2 dígitos | `03` |
| `dd` | Día de 2 dígitos | `09` |
| `HH` | Hora de 2 dígitos (24 h) | `07` |
| `mm` | Minuto de 2 dígitos | `05` |
| `ss` | Segundo de 2 dígitos | `00` |

**Ejemplos de funcionamiento:**

| Patrón | Salida |
|---|---|
| `dd/MM/yyyy HH:mm` | `09/03/2026 07:05` |
| `yyyy-MM-dd` | `2026-03-09` |
| `dd.MM.yyyy HH:mm:ss` | `09.03.2026 07:05:00` |

Los tokens siguen los símbolos de campos de fecha de Unicode LDML (CLDR) y **distinguen entre mayúsculas y minúsculas**. Los patrones que contienen los tokens parecidos `YYYY`, `DD` o `hh` (habituales en las convenciones de Moment o de 12 horas) se rechazan directamente: en LDML significan otra cosa (`YYYY` es el año de numeración de semanas, `DD` es el día del año), así que aceptarlos corrompería en silencio las fechas que ve el votante. Use `yyyy`, `dd` y `HH` en su lugar.

Un patrón no válido (vacío, sin ningún token reconocido o con uno de los tokens rechazados anteriormente) se **rechaza al guardar**: en la pestaña Localización con una notificación de error, y en el campo de formato personalizado de la pestaña Datos con un error debajo del campo, de modo que nunca llega a guardarse. La misma validación se aplica a ambos.

---

## Comportamiento y orden de resolución {#behavior-and-resolution-order}

Para cada fecha visible para el votante, el formato se resuelve en este orden:

1. **Anulación por idioma**: `votingPortalDateTimeFormat` en el idioma activo del votante, si existe y es válida.
2. **Formato del evento**: el preajuste o el patrón personalizado seleccionado en la pestaña Datos.
3. **Legacy GB 24h**: el respaldo fijo.

La anulación es por idioma: un idioma sin anulación usa el preajuste del evento.

**Respaldo sin errores visibles:** si una anulación que llega igualmente a un votante está vacía, no se reconoce o no puede formatear (por ejemplo, datos antiguos anteriores a la validación al guardar), el portal de votación usa en silencio el formato del evento para ese idioma. Del mismo modo, un patrón personalizado para todo el evento que ya no es válido recurre a **Legacy GB 24h**. **No se muestra ningún error al votante**: solo se registra un `console.warn`, y los demás idiomas no se ven afectados.

---

## Elementos afectados {#affected-surfaces}

El formato configurado se aplica a las fechas visibles para el votante que muestra el portal de votación:

- **Tarjetas de elección**: las fechas de apertura y cierre del período que aparecen en la lista de elecciones.
- **Localizador de papeletas**: las marcas de tiempo de la pestaña **Registros**.

> **Nota:** las marcas de tiempo del localizador de papeletas se mostraban antes como una cadena UTC sin formato. Con esta función, se muestran en el formato configurado (en hora local), de forma coherente con el resto del portal de votación.

Este ajuste afecta solo a la **visualización**; los instantes almacenados no cambian.
