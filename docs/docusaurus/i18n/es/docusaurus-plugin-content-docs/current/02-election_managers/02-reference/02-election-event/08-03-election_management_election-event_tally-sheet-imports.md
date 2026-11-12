---
sidebar_position: 8.3
title: Importaciones de actas de escrutinio
description: "Las importaciones de actas de escrutinio permiten a los administradores electorales cargar resultados por recinto desde un tabulador externo y revisarlos en conjunto antes de que se conviertan en versiones activas del acta de escrutinio."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Importaciones de actas de escrutinio

Las importaciones de actas de escrutinio permiten a los administradores electorales cargar resultados por recinto desde un tabulador externo y revisarlos en conjunto antes de que se conviertan en versiones activas del acta de escrutinio.

Una importación tiene el alcance de un proceso electoral. Un archivo cargado puede generar muchas versiones pendientes de actas de escrutinio en distintas áreas, preguntas y elecciones, y en un canal de votación seleccionado. Guardar una importación no cambia los resultados activos. Los resultados activos del acta de escrutinio solo cambian cuando se aprueban las versiones generadas.

En el portal de administración, abra un proceso electoral y seleccione la pestaña **Importación de actas**. La pestaña está disponible para los usuarios con `tally-sheet-import-view`.

## Permisos {#permissions}

La función usa estos permisos:

- `tally-sheet-import-view` para ver el historial de importaciones, el detalle de cada importación, los elementos importados generados, los metadatos del archivo de origen y los enlaces al origen de las versiones importadas
- `tally-sheet-import-create` para cargar, previsualizar y guardar importaciones para su revisión
- `tally-sheet-import-review` para aprobar o desaprobar una importación guardada
- `tally-recount-execute` para iniciar manualmente un recuento de una sesión de recuento completada

## Formato de archivo compatible {#supported-file-format}

STEP usa un formato CSV canónico independiente del fabricante para las importaciones de actas de escrutinio.

El CSV canónico tiene formato largo, con una fila por cada valor de campo:

```csv
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,WARD 15 SUB 01,1,total_votes,,111
PAPER,WARD 15 SUB 01,1,total_valid_votes,,111
PAPER,WARD 15 SUB 01,1,implicit_invalid,,0
PAPER,WARD 15 SUB 01,1,explicit_invalid,,0
PAPER,WARD 15 SUB 01,1,total_blank_votes,,0
PAPER,WARD 15 SUB 01,1,census,,111
PAPER,WARD 15 SUB 01,1,candidate_votes,1,2
PAPER,WARD 15 SUB 01,1,candidate_votes,2,1
```

Las filas se agrupan en urnas por:

```text
channel + area_name + contest_external_id
```

El canal de importación seleccionado debe coincidir con todos los valores de `channel` del CSV.

Cada urna debe incluir exactamente una fila para cada campo escalar: `total_votes`, `total_valid_votes`, `implicit_invalid`, `explicit_invalid`, `total_blank_votes` y `census`. También debe incluir una fila `candidate_votes` para cada candidato de la pregunta de STEP que coincida.

Los archivos cargados se guardan como documentos normales de STEP. La interfaz y la CLI envían un hash SHA-256 del archivo de origen al cargarlo, y las acciones de importación verifican ese hash antes de analizar o guardar la importación. El registro de la importación guarda tanto el hash del archivo de origen original como el hash del CSV canónico usado para la validación y la revisión.

## Columnas {#columns}

`channel` es el canal de votación de STEP para las versiones generadas del acta de escrutinio, por ejemplo `PAPER`.

`area_name` debe coincidir exactamente con el nombre de un área existente en el proceso electoral.

`contest_external_id` debe coincidir con el ID externo de la pregunta configurado en STEP. Como el formato CSV canónico no tiene una columna de elección, **los IDs externos de las preguntas deben ser únicos dentro de un proceso electoral** para que la importación de actas resuelva correctamente las filas. Si un proceso electoral tiene varias elecciones que reutilizan el mismo ID externo para preguntas distintas, la importación fallará con un error de coincidencia ambigua. Antes de importar, cambie el nombre de los IDs externos duplicados.

`field` debe ser uno de los siguientes valores:

- `candidate_votes`
- `total_blank_votes`
- `implicit_invalid`
- `explicit_invalid`
- `total_valid_votes`
- `total_votes`
- `census`

`candidate_external_id` es obligatorio solo cuando `field` es `candidate_votes`. Debe coincidir con un ID externo de candidato de la pregunta correspondiente.

`value` debe ser un número entero no negativo.

## Categorías de votos {#vote-buckets}

Las filas de votos de candidatos se suman en los resultados de los candidatos.

`total_blank_votes` se considera parte de los votos válidos.

`explicit_invalid` corresponde a una opción de voto inválido intencionado en STEP.

`implicit_invalid` corresponde a los votos inválidos causados por las reglas del recuento, como los votos excesivos y los votos insuficientes que no están en blanco.

El importador valida estas invariantes por cada urna:

```text
total_invalid = implicit_invalid + explicit_invalid
total_valid_votes = sum(candidate_votes) + total_blank_votes
total_votes = total_valid_votes + total_invalid
total_votes <= census
```

## Ciclo de vida de la importación {#import-lifecycle}

La previsualización valida el archivo y compara cada urna entrante con la última versión aprobada de la misma elección, área, pregunta y canal. La previsualización muestra los recuentos de urnas nuevas, modificadas, sin cambios y con errores de validación.

Guardar crea un registro de importación inmutable y versiones `PENDING` del acta de escrutinio para las urnas nuevas o modificadas. Las urnas sin cambios siguen visibles en el detalle de la importación, pero no necesitan versiones generadas duplicadas.

Aprobar todo aprueba las versiones generadas si las actas aprobadas de referencia no han cambiado y no existe una versión más reciente no eliminada para ninguna urna afectada. Si se encuentra un conflicto, la aprobación se detiene y las filas en conflicto se marcan.

Desaprobar todo marca las versiones generadas y los elementos de la importación como desaprobados. Las versiones aprobadas del acta de escrutinio que ya existían siguen activas.

Los registros de importación pueden tener estos estados:

- `PENDING_REVIEW`
- `APPROVED`
- `DISAPPROVED`
- `FAILED_VALIDATION`
- `CONFLICTED`

Los elementos de importación pueden tener estos estados:

- `PENDING_REVIEW`
- `APPROVED`
- `DISAPPROVED`
- `CONFLICTED`

La primera implementación revisa una importación completa a la vez. No hay un flujo de aprobación parcial ni de filas seleccionadas.

Si la política de recuento automático del proceso electoral está habilitada, al aprobar una importación solo se inician tareas de recuento para las elecciones en las que los elementos aprobados generaron nuevas versiones del acta de escrutinio. Previsualizar, guardar, desaprobar o aprobar una importación que solo contiene filas sin cambios no inicia un recuento.

El archivo de origen sigue enlazado desde el historial de importaciones para su descarga y auditoría posteriores.

## Diferencias de importación y trazabilidad {#import-diff-and-traceability}

Cada elemento de importación representa una urna, identificada por elección, área, pregunta y canal. La vista de detalle guarda y muestra dos fragmentos CSV canónicos por urna:

- `previous_csv`, generado a partir del último acta de escrutinio aprobado en el momento de la previsualización
- `incoming_csv`, generado a partir de los datos del origen importado

Las filas se clasifican como:

- `NEW` cuando no hay una referencia aprobada para la urna
- `CHANGED` cuando el contenido importado difiere de la referencia aprobada
- `UNCHANGED` cuando el contenido importado coincide con la referencia aprobada

Solo las filas `NEW` y `CHANGED` generan versiones pendientes del acta de escrutinio. Las versiones importadas del acta conservan una referencia `import_id`, de modo que el historial de versiones pueda enlazar con la importación de origen, mostrar el estado de la importación y descargar el archivo XML o CSV de origen original.

## CLI {#cli}

El mismo flujo está disponible en `step-cli`, en `cli step tally-sheet`:

- `import-preview`
- `import-create`
- `import-review`
- `import-list`
- `import-show`
- `import-download-source`
- `recount`
