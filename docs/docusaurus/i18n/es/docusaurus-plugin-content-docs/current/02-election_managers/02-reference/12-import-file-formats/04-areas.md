---
id: import_format_areas
title: Archivo de áreas
sidebar_position: 4
description: "El formato del archivo de áreas: las seis columnas, la política de votación anticipada, un archivo de ejemplo y lo que hace la importación con cada fila."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de áreas es un archivo de texto con una fila para cada área. Usted lo crea, por ejemplo en
una hoja de cálculo. Lo importa en un proceso electoral, en la pestaña **Áreas**, con **Importar**. Para
los pasos, consulte [Importar áreas (alternativa)](../../03-procedures/02-event.md#import-areas-alternative).

Cada fila crea un área nueva. La importación no cambia las áreas que ya existen. Importe las áreas antes
que los votantes. El [archivo de votantes](03-voters.md) se refiere a las áreas por su nombre.

## Campos {#fields}

El archivo no tiene fila de encabezado. Las seis columnas deben estar en este orden. La tabla muestra
cada columna. Los nombres de encabezado de la tabla vienen del formato de origen. La importación no los lee.

| Columna | Nombre del encabezado (no se lee) | Obligatorio | Valores permitidos | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- |
| 1 | `EMB_ID` | Sí | Texto | El **Nombre** del área. El archivo de votantes se refiere a este nombre. | `North` |
| 2 | `CTRY_CODE` | No | Cualquiera | No se usa. La importación lo ignora. Déjelo vacío. | |
| 3 | `EMB_CODE` | No | Cualquiera | No se usa. La importación lo ignora. Déjelo vacío. | |
| 4 | `AREANAME` | No | Texto | La **Descripción** del área. | `North District` |
| 5 | `isdelete` | Sí | `0` | La marca de importación. Solo `0` importa la fila. Cualquier otro valor, o una celda vacía, omite la fila. | `0` |
| 6 | `EARLY_VOTING_POLICY` | No | `allow_early_voting` o `no_early_voting` | La política de votación anticipada del área. Solo `allow_early_voting` permite la votación anticipada. Cualquier otro valor, o una celda vacía, significa `no_early_voting`. | `no_early_voting` |

### Nombre {#name}

- La importación usa la columna 1 tal como está. No quita los espacios alrededor del nombre.
- La importación no comprueba que la columna 1 no esté vacía. Una fila con la columna 1 vacía crea un
  área con nombre vacío. Compruebe cada fila antes de importar.
- La importación no comprueba si hay nombres duplicados. Cada fila crea un área nueva, aunque el nombre
  ya exista. La importación de votantes elige entonces una de las áreas con ese nombre. La elección no
  está definida.

:::caution PRECAUCIÓN: importar de nuevo el archivo de áreas
Si importa el mismo archivo otra vez, las áreas se crean de nuevo. Compruebe la pestaña **Áreas** antes
de importar el archivo por segunda vez.
:::

### Preguntas {#contests}

Una importación vincula cada área nueva con **todas** las preguntas del proceso electoral que existen en
el momento de la importación. Quite las preguntas que no pertenecen a un área. Consulte [Reglas](#rules).

### Votación anticipada {#early-voting}

La importación no comprueba los canales de votación del proceso electoral. Un área que recibe
`allow_early_voting` lo mantiene, aunque el canal **Votación anticipada** esté desactivado.

En el formulario del área, la casilla **Permitir Votación Temprana** está deshabilitada mientras el canal
**Votación anticipada** esté desactivado.

## Archivo de ejemplo {#example-file}

```csv
North,,,North District,0,no_early_voting
South,,,South District,0,allow_early_voting
Old Area,,,Old District,1,no_early_voting
```

La primera fila crea el área **North**. La votación anticipada no está permitida en esta área. La segunda
fila crea el área **South**. La votación anticipada está permitida en esta área. La tercera fila no se
importa, porque la columna 5 es `1`.

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Fila de encabezado | No añada una. La importación lee cada fila como datos. Una fila de encabezado tiene `isdelete` en la columna 5, así que la importación la omite. No confíe en esto. |
| Columnas | Seis columnas en cada fila. |
| Separador | Una coma. Un campo que contiene una coma debe ir entre comillas dobles. |
| Marca de importación | La columna 5 debe ser exactamente `0`. Un espacio u otro carácter omite la fila. |
| Política de votación anticipada | La columna 6 debe ser exactamente `allow_early_voting`. Las mayúsculas o los espacios extra dan `no_early_voting`. |
| Verificación de integridad | Si escribe un valor SHA-256, la importación lo compara con el hash del archivo. La comparación ignora mayúsculas y minúsculas. No ignora los espacios. |
| Sin verificación de integridad | Si el campo está vacío, el portal le pide que confirme. Entonces la importación se ejecuta sin comprobación. |
| Todo o nada | La importación guarda todas las áreas y vínculos en una sola transacción. Si ocurre un error, no se guarda ninguna área. |
| Permiso | El usuario necesita el permiso **Editar Área**. |
| Tamaño del archivo | El límite de subida de su entorno. La importación no tiene límite de filas. |

### Quitar una pregunta de un área {#remove-a-contest-from-an-area}

1. En la pestaña **Áreas**, haga clic en el icono de editar del área.
2. En **Preguntas**, quite las preguntas que no pertenecen al área.
3. Haga clic en **Guardar**.

Al guardar, se reemplazan todos los vínculos de preguntas del área por las preguntas que quedan en el campo.

## Qué hace la importación {#what-the-import-does}

1. El portal sube el archivo. El botón **Importar** se habilita cuando termina la subida.
2. Escriba el valor SHA-256 del archivo en **Verificación de Integridad (SHA 256)**. Este paso es opcional.
   Si el campo está vacío y hace clic en **Importar**, el portal pregunta **Importar Sin Verificación de Integridad?**.
   Haga clic en **Sí, Importar Sin Verificación de Integridad** para continuar, o en **Volver** para parar.
3. El servidor comprueba el hash, si ha escrito uno. Un hash que no coincide detiene la importación.
4. El servidor lee cada fila. Omite cada fila en la que la columna 5 no es `0`. Para cada una de las otras
   filas, crea un área con un ID nuevo. La política de votación anticipada viene de la columna 6.
5. El servidor vincula cada área nueva con todas las preguntas del proceso electoral.
6. El servidor guarda todas las áreas y vínculos en una transacción. Si un paso falla, el servidor no guarda
   nada.
7. El portal muestra **Áreas Importadas Exitosamente** y actualiza la lista. La importación no envía ningún
   mensaje.

La importación de áreas no inicia una tarea. No hay registro de tarea para ella. El resultado aparece solo
en el portal.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde | Significado y acción |
| --- | --- | --- |
| Áreas Importadas Exitosamente | Notificación | La importación ha terminado. Compruebe la pestaña **Áreas**. |
| Error al importar Áreas | Notificación | La importación falló. No se guardó ninguna área. Compruebe el archivo y el hash. |
| Fichero subido al servidor - pero no importado aún | Notificación | El archivo está en el servidor. Haga clic en **Importar**. |
| Error subiendo el fichero | Notificación | La subida falló. Seleccione el archivo de nuevo. |
| Importar Sin Verificación de Integridad? | Diálogo | No ha escrito un hash. Confirme solo si el archivo es el archivo correcto. |
| `Failed to verify the integrity: Hash of voters file: <hash of the file> does not match with the input hash: <typed hash>` | Error del servidor | El hash que escribió no es el hash del archivo. Compruebe el hash y el archivo. El texto dice "voters file" porque el código es compartido. |
| `Failed to verify the integrity: <reason>` | Error del servidor | El servidor no pudo leer ni calcular el hash del archivo. Suba el archivo de nuevo. |
| `document not found` | Error del servidor | El servidor no tiene el archivo. Suba el archivo de nuevo. |
| `Error reading CSV record` | Error del servidor | El servidor no pudo leer una fila. Compruebe que cada fila tiene seis columnas. |

## Páginas relacionadas {#related-pages}

- [Importar áreas (alternativa)](../../03-procedures/02-event.md#import-areas-alternative): los pasos para importar el archivo.
- [Archivo de votantes](03-voters.md): el archivo de votantes se refiere a las áreas por su nombre.
- [Formatos de archivo de importación](index.md): todos los archivos de importación.
