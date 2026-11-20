---
id: import_file_formats
title: Formatos de archivo de importación
description: "Los formatos de los archivos que puede importar en el portal de administración: qué contiene cada archivo, cómo crearlo y ejemplos."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Esta sección indica el formato de cada archivo que el portal de administración puede importar, con
una página para cada archivo. La mayoría de los archivos son archivos que se exportan desde el
portal de administración. Algunos archivos, por ejemplo el archivo de votantes, los crea usted
mismo.

## Resumen {#summary}

| Archivo | Dónde lo importa | Tipo | ¿Lo crea usted mismo? |
| --- | --- | --- | --- |
| [Proceso electoral](01-election-event.md) y sus [elecciones, preguntas y candidatos](02-election-contest-candidate.md) | **Procesos Electorales** > **+** > **Importar Evento Electoral** | `.json`, `.zip`, `.ezip` | No. Use una exportación. |
| [Votantes](03-voters.md) | Proceso electoral > **Votantes** > **Importar** | `.csv`, `.tsv` | Sí |
| [Áreas](04-areas.md) | Proceso electoral > **Áreas** > **Importar** | `.csv` | Sí |
| [Aplicaciones](05-applications.md) | Proceso electoral > **Aprobaciones** > **Importar** | `.csv` | No. Use una exportación. |
| [Plantillas](06-templates.md) | **Plantillas** > **Importar** | `.csv` | No. Use una exportación. |
| [Copia de seguridad del inquilino](07-tenant-backup.md) | **Configuración** > **COPIA DE SEGURIDAD / RESTAURAR** | `.zip` | No. Use una exportación. |
| [Copia de seguridad de la clave privada](08-private-key-backup.md) | Ceremonia de claves y ceremonia de recuento | `.txt` | No. Use el archivo descargado. |
| [Presentación de la elección](09-election-presentation.md) | Elección > **Datos** (campo de archivo en el formulario de la elección) | `.json` | Sí, con cuidado |

Cada panel de importación tiene el campo **Verificación de Integridad (SHA 256)**. Escriba allí el
hash SHA-256 del archivo. La importación rechaza entonces un archivo que alguien ha cambiado. En
esta versión, la importación del proceso electoral no comprueba el hash: consulte
[Archivo del proceso electoral](01-election-event.md). Para obtener el hash de un archivo:

```bash
shasum -a 256 <file>
```

En Windows, use `certutil -hashfile <file> SHA256`.

## Otras importaciones {#other-imports}

- **Importar Candidatos** en la pestaña **Datos** de un proceso electoral lee un diseño de archivo
  fijo. Cada valor se lee desde una posición de columna fija. No es una importación de uso
  general. Para añadir candidatos, use el árbol del proceso electoral o importe un proceso
  electoral.
- La firma del paquete de transmisión y las imágenes y los documentos que añade a los candidatos y
  a los materiales de apoyo no están en un formato de Step.
