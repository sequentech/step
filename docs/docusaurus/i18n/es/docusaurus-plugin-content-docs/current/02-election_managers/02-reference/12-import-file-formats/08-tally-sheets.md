---
id: import_format_tally_sheets
title: Archivo de actas de escrutinio
sidebar_position: 8
description: "El formato del archivo de actas de escrutinio: las columnas, los campos, las reglas aritméticas, los errores de validación y el paso de revisión."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de actas de escrutinio contiene los resultados de la votación en papel, por correo o presencial. Cada fila da un número para un área y una pregunta. Usted crea el archivo, por ejemplo a partir de las actas de recuento de los colegios electorales. Lo importa en el proceso electoral, en la sección **Importación de actas** de la pestaña **Recuento**. Para ver la pantalla, consulte [Importaciones de actas de escrutinio](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md). Para los pasos del recuento, consulte [Realizar la ceremonia de recuento](../../03-procedures/06-tally.md).

La importación tiene dos formatos. Esta página describe **CSV canónico**. El otro formato, **XML**, es el formato de resultados de un sistema externo. La plataforma convierte ese archivo XML al formato CSV canónico y después aplica las mismas comprobaciones.

Necesita estos permisos: **Crear importación de actas de recuento** (`tally-sheet-import-create`) para subir un archivo, y **Revisar importación de actas de recuento** (`tally-sheet-import-review`) para aprobarlo o desaprobarlo.

## Campos {#fields}

El archivo tiene una fila de encabezado. El orden de las columnas no es importante. Las columnas se leen por nombre. El archivo debe tener las seis columnas.

| Columna | Obligatoria | Valores permitidos o formato | Valor predeterminado | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- |
| `channel` | Obligatoria | `PAPER`, `POSTAL` o `IN_PERSON`. Use mayúsculas. | Ninguno | El canal de votación de los resultados. Debe ser el mismo que el canal que selecciona en el panel de importación. | `PAPER` |
| `area_name` | Obligatoria | El **Nombre** de un área del proceso electoral | Ninguno | El área de los resultados. El área debe estar vinculada a la pregunta. | `North` |
| `contest_external_id` | Obligatoria | El **ID externo** de una pregunta | Ninguno | La pregunta de los resultados. | `contest-1` |
| `field` | Obligatoria | Uno de los valores de la tabla siguiente | Ninguno | El tipo de número de la columna `value`. | `census` |
| `candidate_external_id` | Solo para `candidate_votes` | El **ID externo** de un candidato de la pregunta | Vacío | El candidato. Déjelo vacío para los demás campos. | `candidate-1` |
| `value` | Obligatoria | Un número entero, 0 o más | Ninguno | El número. | `1200` |

La importación ignora los espacios al principio y al final de `channel`, `area_name`, `contest_external_id`, `field`, `candidate_external_id` y `value`.

### Valores de `field` {#values-of-field}

La lista está cerrada. Cualquier otro valor da el error `invalid_field`.

| Valor | Obligatorio | Significado |
| --- | --- | --- |
| `census` | Sí | El número de votantes que pueden votar en esta área y pregunta. |
| `total_votes` | Sí | El número de papeletas contadas: válidas, en blanco e inválidas. |
| `total_valid_votes` | Sí | El número de papeletas válidas. Incluye los votos en blanco. |
| `total_blank_votes` | Sí | El número de papeletas en blanco en esta pregunta. |
| `implicit_invalid` | Sí | El número de votos inválidos causados por las reglas de recuento, por ejemplo demasiadas marcas. |
| `explicit_invalid` | Sí | El número de votos inválidos que el votante eligió a propósito. |
| `blank_ballots` | No | El número de papeletas en blanco en todas las preguntas del área y del canal. |
| `candidate_votes` | Una fila por cada candidato | Los votos de un candidato. |

La plataforma calcula `total_invalid` como `implicit_invalid` más `explicit_invalid`. Usted no lo escribe en el archivo.

### Reglas para los números {#rules-for-the-numbers}

La importación comprueba estas reglas para cada área y pregunta. Comprueba el par área-pregunta en un canal.

1. `total_votes` = `total_valid_votes` + `implicit_invalid` + `explicit_invalid`.
2. `total_votes` no es mayor que `census`.
3. La suma de los `candidate_votes` está entre *A* y *A* x *M*. *A* es `total_valid_votes` menos `total_blank_votes`. *M* es el número más alto de marcas que puede tener una papeleta.
   - *M* es el **Votos máximos** de la pregunta. Si la pregunta no tiene valor, *M* es 1.
   - Si el algoritmo de recuento es `cumulative`, *M* es **Votos máximos** multiplicado por el número de casillas de cada candidato.
   - En una pregunta normal donde cada votante elige un candidato, la suma es exactamente *A*.
4. `blank_ballots` (si lo indica) tiene el mismo valor en cada fila de la misma área y canal. No es mayor que el `total_blank_votes` más bajo de las preguntas de esa área. No es menor que la suma de los `total_blank_votes` de las preguntas, menos (el número de preguntas menos 1) multiplicado por el `total_votes` más alto. La plataforma también lo compara con las actas aprobadas de las preguntas que no están en el archivo.
5. Si estos límites permiten solo un valor, y no indicó `blank_ballots`, la plataforma rellena ese valor.

Otras reglas:

- Cada candidato de la pregunta debe tener un **ID externo**, y los IDs externos deben ser únicos en la pregunta.
- El archivo debe tener una fila `candidate_votes` para cada candidato de la pregunta, también si los votos son `0`.
- El ID externo de la pregunta debe ser único en el proceso electoral.
- No puede importar actas de escrutinio de una pregunta aclamada.

## Reglas del archivo {#rules-of-the-file}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.csv` (o `.xml` para el otro formato). El separador del archivo CSV es una coma. |
| Encabezado | Obligatorio. |
| Codificación | UTF-8. |
| Orden de las filas | No es importante. |
| Filas duplicadas | Dos filas con el mismo `field` para una área y pregunta dan un error. Lo mismo ocurre con dos filas `candidate_votes` para un mismo candidato. |
| Tamaño | 50 MB (52 428 800 bytes) como máximo. |
| Comprobación de integridad | El portal de administración calcula el hash SHA-256 del archivo. Usted no lo escribe. El servidor lo comprueba. |
| Canal | Una importación tiene un canal. El canal de cada fila debe ser el mismo que el canal que selecciona. |

## Archivo de ejemplo {#example-file}

```csv
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,North,contest-1,census,,1200
PAPER,North,contest-1,total_votes,,815
PAPER,North,contest-1,total_valid_votes,,790
PAPER,North,contest-1,total_blank_votes,,10
PAPER,North,contest-1,implicit_invalid,,9
PAPER,North,contest-1,explicit_invalid,,16
PAPER,North,contest-1,candidate_votes,candidate-1,480
PAPER,North,contest-1,candidate_votes,candidate-2,300
PAPER,South,contest-1,census,,900
PAPER,South,contest-1,total_votes,,600
PAPER,South,contest-1,total_valid_votes,,590
PAPER,South,contest-1,total_blank_votes,,5
PAPER,South,contest-1,implicit_invalid,,4
PAPER,South,contest-1,explicit_invalid,,6
PAPER,South,contest-1,candidate_votes,candidate-1,285
PAPER,South,contest-1,candidate_votes,candidate-2,300
```

En el área North: 790 + 9 + 16 = 815 (`total_votes`). 480 + 300 = 780, y 790 - 10 = 780 (votos de los candidatos). 815 es menor que 1200 (`census`). Compruebe su propio archivo con las mismas sumas.

## Qué hace la importación {#what-the-import-does}

La importación tiene dos pasos: una vista previa y un guardado. No cambia nada en los resultados hasta que un revisor aprueba la importación.

1. En el proceso electoral, abra la pestaña **Recuento**. En **Importación de actas**, haga clic en **Importar actas de escrutinio**.
2. Seleccione el **Formato** (**CSV canónico**) y el **Canal**.
3. Suelte el archivo en el panel. Si una importación anterior tiene el mismo archivo, el portal muestra la advertencia "Este hash de archivo de origen ya aparece en una importación de acta de escrutinio anterior." y el botón **Abrir existente**. La advertencia no le impide continuar.
4. Haga clic en **Previsualizar**. La plataforma lee el archivo, encuentra cada área, pregunta y candidato, comprueba las reglas y muestra las urnas. El panel muestra los recuentos: **Importadas**, **Modificadas**, **Nuevas**, **Sin cambios**, **En conflicto** y **Errores**.
5. Si el panel muestra errores, corrija el archivo y haga clic en **Previsualizar** de nuevo. El botón **Guardar importación** permanece desactivado mientras haya errores.
6. Haga clic en **Guardar importación**. Aparece el mensaje "Se ha creado la importación del acta de escrutinio". La importación tiene el estado **Pendiente de revisión**. La plataforma crea una versión nueva y pendiente de un acta de escrutinio para cada urna nueva o modificada. No crea nada para una urna sin cambios.
7. Un usuario con el permiso de revisión abre la importación y hace clic en **Revisar**. El usuario compara los valores anteriores y los nuevos.
8. El usuario hace clic en **Aprobar** o en **Desaprobar**.

En la vista previa, cada urna recibe una de estas etiquetas: **Nueva** (no hay ningún acta aprobada antes), **Modificada** (los números no son iguales que en el último acta aprobada) o **Sin cambios**.

Después de **Aprobar**, las nuevas actas de escrutinio quedan aprobadas. La plataforma marca como eliminadas todas las demás versiones de la misma urna (misma elección, área, pregunta y canal), sea cual sea su estado. El recuento usa entonces las actas aprobadas. Después de **Desaprobar**, las nuevas actas de escrutinio quedan desaprobadas y los resultados no cambian.

Si alguien aprobó otra acta de escrutinio para la misma urna después de que usted guardara la importación, la importación recibe el estado **En conflicto**. Aparece el mensaje "La importación tiene conflictos de línea base desactualizados". No se aprueba nada. Importe el archivo de nuevo.

### Estados de una importación {#statuses-of-an-import}

| Estado | Significado |
| --- | --- |
| **Pendiente de revisión** | La importación está guardada. Un revisor debe aprobarla o desaprobarla. |
| **Aprobada** | Las nuevas actas de escrutinio están aprobadas. |
| **Desaprobada** | Las nuevas actas de escrutinio están rechazadas. |
| **Validación fallida** | El archivo tenía errores al guardar la importación. La importación no tiene actas de escrutinio. |
| **En conflicto** | Otra acta de escrutinio cambió después de guardar la importación. |

### Mensajes en el portal de administración {#messages-in-the-admin-portal}

| Mensaje | Significado y acción |
| --- | --- |
| Selecciona un archivo de importación antes de previsualizarlo | Suelte primero un archivo. |
| No se pudo crear la URL de subida, No se pudo subir el archivo de importación | La subida falló. Inténtelo de nuevo. |
| No se pudo previsualizar la importación, La respuesta de la previsualización estaba vacía | La vista previa falló. Lea el mensaje, corrija el archivo e inténtelo de nuevo. |
| Se ha creado la importación del acta de escrutinio | La importación está guardada y espera revisión. |
| No se pudo crear la importación | El guardado falló. |
| Se ha aprobado la importación, Se ha desaprobado la importación | La revisión ha terminado. |
| La importación tiene conflictos de línea base desactualizados | Vea el texto anterior. |
| No se pudo revisar la importación | La revisión falló. |

## Errores de validación {#validation-errors}

Cada error tiene un código. El panel muestra el texto del mensaje para el código. El error muestra el canal, el área, la pregunta y el campo cuando se conocen.

| Código | Significado | Acción |
| --- | --- | --- |
| `invalid_csv_row` | Una fila tiene un número incorrecto de columnas o no se puede leer. | Corrija la fila. |
| `invalid_channel` | El `channel` no es `PAPER`, `POSTAL` ni `IN_PERSON`. | Use uno de estos valores en mayúsculas. |
| `selected_channel_mismatch` | El `channel` de una fila no es el canal seleccionado en el panel. | Seleccione el mismo canal que el archivo, o divida el archivo. |
| `invalid_field` | El `field` no está en la lista. | Corrija el nombre. |
| `invalid_numeric_value` | El `value` no es un número entero de 0 o más. | Corrija el valor. |
| `missing_candidate_external_id` | Una fila `candidate_votes` no tiene `candidate_external_id`. | Añada el ID externo. |
| `duplicate_candidate_votes` | Dos filas dan los votos de un mismo candidato. | Elimine una fila. |
| `duplicate_scalar_field` | Dos filas dan el mismo campo para una área y pregunta. | Elimine una fila. |
| `missing_scalar_field` | Un campo obligatorio no tiene fila. | Añada la fila. |
| `unresolved_ballot_box` | La plataforma no encontró el área, la pregunta o un candidato, o el área no está vinculada a la pregunta, o un candidato no tiene ID externo, o un candidato no tiene fila `candidate_votes`. | Lea el mensaje. Corrija el archivo o el proceso electoral. |
| `tally_sheet_not_allowed_for_acclaimed_contest` | La pregunta es aclamada. | Elimine la pregunta del archivo. |
| `unknown_counting_algorithm` | El algoritmo de recuento de la pregunta no se conoce. | Corrija la pregunta. |
| `invalid_total_invalid` | El `total_invalid` no es `implicit_invalid` más `explicit_invalid`. | Corrija los números. |
| `invalid_total_valid_votes` | Los votos de los candidatos están fuera de los límites permitidos (regla 3). | Corrija los números. |
| `invalid_total_votes` | El `total_votes` no es `total_valid_votes` más los votos inválidos (regla 1). | Corrija los números. |
| `total_votes_exceeds_census` | El `total_votes` es mayor que `census` (regla 2). | Corrija los números. |
| `inconsistent_blank_ballots` | Las filas de una misma área y canal dan valores distintos para `blank_ballots`. | Use un solo valor. |
| `blank_ballots_out_of_bounds` | El `blank_ballots` está fuera de los límites (regla 4). | Corrija el valor. |

Los mensajes de `unresolved_ballot_box` son:

- `Area '<name>' not found`
- `Contest external id '<id>' not found`
- `Area '<name>' is not assigned to contest external id '<id>'`
- `Contest external id '<id>' has candidate(s) without an external_id required for import: <names>`
- `Contest external id '<id>' has duplicate candidate external_id value(s): <ids>`
- `Import for contest external id '<id>' is missing candidate_votes rows for candidate external_id value(s): <ids>`
- `Candidate external id '<id>' not found in contest external id '<id>'`

El texto de `tallysheet.inputError` en el portal de administración da el mensaje para estos códigos: `invalid_total_valid_votes`, `total_votes_exceeds_census`, `invalid_total_invalid`, `invalid_total_votes`, `unknown_counting_algorithm`, `inconsistent_blank_ballots` y `blank_ballots_out_of_bounds`. Para los demás códigos, el portal muestra el mensaje del servidor en inglés.

Un archivo de más de 50 MB se rechaza con el error "El documento ... es demasiado grande". Un hash que no coincide se rechaza con el error "El SHA-256 del archivo de origen subido no coincide".

El portal muestra el mensaje de error de la petición fallida. El servidor responde con el estado 413 para un archivo demasiado grande y con el estado 400 para un hash que no coincide.

## Páginas relacionadas {#related-pages}

- [Importaciones de actas de escrutinio](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md): la sección **Importación de actas**.
- [Realizar la ceremonia de recuento](../../03-procedures/06-tally.md): el recuento usa las actas de escrutinio aprobadas.
- [Archivo de áreas](04-areas.md): las áreas que nombra el archivo de actas de escrutinio.
- [Formatos de archivos de importación](index.md): todos los archivos de importación.
