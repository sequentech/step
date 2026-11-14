---
id: import_format_areas
title: Archivo de áreas
sidebar_position: 4
description: "El formato del archivo de áreas: las cinco columnas, un archivo de ejemplo, la actualización de áreas padre y lo que hace la importación con cada fila."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de áreas es un archivo de texto con una fila para cada área. Usted lo prepara, por ejemplo, en una hoja de cálculo. Lo importa en un proceso electoral, en la pestaña **Áreas**, con **Importar**. Para los pasos, consulte [Importar áreas (alternativa)](../../procedures/02-event.md#import-areas-alternative).

Cada fila crea una nueva área. La importación no cambia las áreas que ya existen. Importe las áreas antes que los votantes. El [archivo de votantes](03-voters.md) hace referencia a las áreas por su nombre.

## Campos {#fields}

El archivo no tiene fila de encabezado. La importación lee las columnas 1, 4 y 5. No lee las columnas 2 y 3. Los nombres de encabezado de la tabla proceden del formato de origen. La importación no los lee.

| Columna | Nombre del encabezado (no se lee) | Obligatoria | Valores permitidos | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- |
| 1 | `EMB_ID` | Sí | Texto | El **Nombre** del área. El archivo de votantes hace referencia a este nombre. | `North` |
| 2 | `CTRY_CODE` | No | Cualquiera | No se usa. La importación lo ignora. Déjelo vacío. | |
| 3 | `EMB_CODE` | No | Cualquiera | No se usa. La importación lo ignora. Déjelo vacío. | |
| 4 | `AREANAME` | No | Texto | La **Descripción** del área. | `North District` |
| 5 | `isdelete` | Sí | `0` | La marca de importación. Solo `0` importa la fila. Cualquier otro valor, o una celda vacía, omite la fila. | `0` |

### Nombre {#name}

- La importación usa la columna 1 tal como está. No quita los espacios alrededor del nombre.
- La importación no comprueba que la columna 1 no esté vacía. Una fila con la columna 1 vacía crea un área con un nombre vacío. Compruebe cada fila antes de importar.
- La importación no comprueba si hay nombres duplicados. Cada fila crea una nueva área, aunque el nombre ya exista. La importación de votantes elige entonces una de las áreas con ese nombre. La elección no está definida.

:::caution PRECAUCIÓN: importar de nuevo el archivo de áreas
Si importa el mismo archivo otra vez, las áreas se crean de nuevo. Compruebe la pestaña **Áreas** antes de importar el archivo por segunda vez.
:::

### Preguntas {#contests}

Una importación vincula cada nueva área con **todas** las preguntas del proceso electoral que existen en el momento de la importación. Quite las preguntas que no pertenecen a un área. Consulte [Reglas](#rules).

## Archivo de ejemplo {#example-file}

```csv
North,,,North District,0
South,,,South District,0
Old Area,,,Old District,1
```

La primera fila crea el área **North**. La segunda fila crea el área **South**. La tercera fila no se importa, porque la columna 5 es `1`.

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Fila de encabezado | No añada una. La importación lee cada fila como datos. Una fila de encabezado tiene `isdelete` en la columna 5, por lo que la importación la omite. No dependa de esto. |
| Columnas | El mismo número de columnas en cada fila. Use al menos cinco columnas. Si una fila tiene un número distinto de columnas, la importación se detiene con `Error reading CSV record`. |
| Separador | Una coma. Un campo que contiene una coma debe ir entre comillas dobles. |
| Marca de importación | La columna 5 debe ser exactamente `0`. Un espacio u otro carácter omite la fila. |
| Verificación de integridad | Si escribe un valor SHA-256, la importación lo compara con el hash del archivo. La comparación no distingue entre mayúsculas y minúsculas. No ignora los espacios. |
| Sin verificación de integridad | Si el campo está vacío, el portal le pide que confirme. La importación continúa entonces sin comprobación. |
| Todo o nada | La importación guarda todas las áreas y vínculos en una sola transacción. Si se produce un error, no se guarda ningún área. |
| Permiso | El usuario necesita el permiso **Importar Área** para ver el botón **Importar**. El servidor también comprueba el permiso **Editar Área**. |
| Tamaño del archivo | El límite de subida de su entorno. La importación no tiene límite de filas. |

### Quitar una pregunta de un área {#remove-a-contest-from-an-area}

1. En la pestaña **Áreas**, haga clic en el icono de editar del área.
2. En **Preguntas**, quite las preguntas que no pertenecen al área.
3. Haga clic en **Guardar**.

El guardado reemplaza todos los vínculos de preguntas del área con las preguntas que quedan en el campo.

## Qué hace la importación {#what-the-import-does}

1. El portal sube el archivo. El botón **Importar** se habilita cuando termina la subida.
2. Escriba el valor SHA-256 del archivo en **Verificación de Integridad (SHA 256)**. Este paso es opcional. Si el campo está vacío y hace clic en **Importar**, el portal pregunta **Importar Sin Verificación de Integridad?**. Haga clic en **Sí, Importar Sin Verificación de Integridad** para continuar, o en **Volver** para detenerse.
3. El servidor comprueba el hash, si escribió uno. Una diferencia detiene la importación.
4. El servidor lee cada fila. Omite cada fila en la que la columna 5 no es `0`. Para cada una de las otras filas, crea un área con un ID nuevo.
5. El servidor vincula cada nueva área con todas las preguntas del proceso electoral.
6. El servidor guarda todas las áreas y vínculos en una sola transacción. Si un paso falla, el servidor no guarda nada.
7. El portal muestra **Áreas Importadas Exitosamente** y actualiza la lista. La importación no envía ningún mensaje.

La importación de áreas no inicia ninguna tarea. No hay registro de tarea para ella. El resultado se muestra solo en el portal.

## Actualizar áreas padre {#upsert-parent-areas}

Use la acción **Upsert Areas** de la pestaña **Áreas** para asignar el área padre de áreas que ya existen. Esta acción no crea áreas. Importe primero las áreas. La acción necesita el permiso **Insertar o Actualizar Área**. El servidor también comprueba el permiso **Editar Área**.

El archivo para esta acción es diferente del archivo de áreas:

- La primera fila es siempre el encabezado. La importación la omite, por lo que el archivo debe tener una fila de encabezado.
- La columna 1 es el **Nombre** del área. La importación cambia solo las áreas con este nombre.
- La columna 2 es el **Nombre** del área padre. Déjela vacía para un área sin padre.
- Cada fila debe tener el mismo número de columnas que la fila de encabezado.
- Una fila cuya columna 1 no coincide con ningún nombre de área se omite. La importación no muestra ningún error.
- Si la columna 2 está vacía, o no coincide con ningún nombre de área, la importación quita el padre del área.
- El campo **Verificación de Integridad (SHA 256)** no se usa. La importación no comprueba el hash.

## Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde | Significado y acción |
| --- | --- | --- |
| Áreas Importadas Exitosamente | Notificación | La importación terminó. Compruebe la pestaña **Áreas**. |
| Error al importar Áreas | Notificación | La importación falló. No se guardó ningún área. Compruebe el archivo y el hash. |
| Fichero subido al servidor - pero no importado aún | Notificación | El archivo está en el servidor. Haga clic en **Importar**. |
| Error subiendo el fichero | Notificación | La subida falló. Seleccione el archivo de nuevo. |
| Importar Sin Verificación de Integridad? | Diálogo | No escribió un hash. Confirme solo si el archivo es el correcto. |
| `Failed to verify the integrity: Hash of voters file: <hash of the file> does not match with the input hash: <typed hash>` | Error del servidor | El hash escrito no es el hash del archivo. Compruebe el hash y el archivo. El texto dice `voters file` en el código. Es posible que el portal no muestre este mensaje. |
| `Failed to verify the integrity: <reason>` | Error del servidor | El servidor no pudo leer ni calcular el hash del archivo. Suba el archivo de nuevo. |
| `document not found` | Error del servidor | El servidor no tiene el archivo. Suba el archivo de nuevo. |
| `Error reading CSV record` | Error del servidor | El servidor no pudo leer una fila. Compruebe que cada fila tiene el mismo número de columnas. |

## Páginas relacionadas {#related-pages}

- [Importar áreas (alternativa)](../../procedures/02-event.md#import-areas-alternative): los pasos para importar el archivo.
- [Archivo de votantes](03-voters.md): el archivo de votantes hace referencia a las áreas por su nombre.
- [Formatos de archivo de importación](index.md): todos los archivos de importación.
