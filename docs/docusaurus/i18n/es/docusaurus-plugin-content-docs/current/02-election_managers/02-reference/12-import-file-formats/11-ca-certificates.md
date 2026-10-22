---
id: import_format_ca_certificates
title: Archivo de certificados de la CA
sidebar_position: 11
description: "El formato del archivo de certificados de la CA: los bloques PEM, las reglas, los campos que muestra el portal para cada certificado y los resultados de la importación."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de certificados de la CA es un archivo de texto en formato PEM. Contiene uno o más certificados de autoridades de certificación (CA). La plataforma usa las CA importadas para validar los certificados de los votantes. Usted importa el archivo en un proceso electoral, en la pestaña **Certificados**, con **Importar certificados** o **Importar**. Usted no crea el archivo. Lo obtiene de la CA o de la fuente de los certificados.

## Antes de importar {#before-you-import}

La pestaña **Certificados** solo se muestra cuando la política de certificados digitales de votantes está habilitada.

1. Abra la pestaña **Datos** del proceso electoral.
2. Establezca **Voter Digital Certificate Policy** en **Habilitado**.
3. Guarde el proceso electoral.

La importación se rechaza mientras la política esté **Deshabilitado**. El usuario necesita el permiso **Editar autoridades de certificación**.

## Qué es el archivo {#what-the-file-is}

| Propiedad | Valor |
| --- | --- |
| Tipo | Texto plano, formato PEM. |
| Contenido | Uno o más bloques de certificado. El otro texto se ignora. |
| Inicio de bloque | La línea `-----BEGIN CERTIFICATE-----`. Los espacios antes o después de la línea se ignoran. |
| Fin de bloque | La línea `-----END CERTIFICATE-----`. Los espacios antes o después de la línea se ignoran. |
| Cuerpo del bloque | El texto Base64 de un certificado X.509, como en un archivo PEM. |
| Un bloque para | Un certificado. |
| Orden | Cualquiera. Cada bloque se importa por separado. |

Ejemplo de la estructura. Las líneas Base64 son marcadores de posición. Este archivo no se importa:

```text
Text before the first certificate is ignored.
-----BEGIN CERTIFICATE-----
MIIB...
...more Base64 lines...
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
MIIB...
-----END CERTIFICATE-----
```

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Bloque sin línea de fin | El bloque se ignora. El portal no muestra ningún mensaje para él. |
| Texto dentro de un bloque | El servidor lee las líneas del bloque como Base64. No añada otro texto dentro de un bloque. Una línea de otro texto da un error para ese certificado. |
| Certificado no válido | Un error solo para este certificado. Los demás certificados se siguen importando. |
| Certificado caducado | No se importa. El resultado es un error. |
| Certificado duplicado | Se omite. Un certificado es duplicado cuando su huella SHA256 ya está en el proceso electoral. |
| Ámbito | Los certificados pertenecen a un proceso electoral. El mismo certificado puede importarse en otro proceso electoral. |
| Ningún bloque en el archivo | La importación se rechaza. |
| Política deshabilitada | La importación se rechaza. No se importa ningún certificado. |
| Eliminar | Un certificado eliminado se quita del proceso electoral. La eliminación se escribe en el registro electoral. |

## Campos que muestra el portal {#fields-shown-in-the-portal}

La pestaña **Certificados** muestra una fila para cada certificado. La tabla lista las columnas de la pestaña.

| Columna | Origen | Significado |
| --- | --- | --- |
| Nombre común | El nombre común (CN) del sujeto. | Si el certificado no tiene CN, el portal muestra el sujeto completo. |
| Tipo | El sujeto y el emisor. | **Raíz** si el sujeto y el emisor son iguales. Si no, **Intermedio**. Un certificado que no está autofirmado también muestra **Intermedio**. |
| CN del emisor | El CN del emisor. | Si el emisor no tiene CN, el portal muestra el emisor completo. |
| Válido desde | El inicio del período de validez. | Solo la fecha, en el formato de fecha del navegador. |
| Vence | El fin del período de validez. | La fecha y un estado: **Válido**, **Por vencer** cuando quedan menos de 30 días, o **Vencido**. |
| Huella SHA256 | El hash SHA-256 del certificado. | Los primeros 24 caracteres, y después `…`. El valor completo se muestra en una información emergente. |

La huella es el hash SHA-256 del certificado, en hexadecimal en mayúsculas, con dos puntos entre cada byte. Forma de ejemplo: `AB:CD:EF:...`.

Haga clic en el icono de ver de una fila para abrir **Detalles de la autoridad de certificación**. Los detalles también muestran:

| Campo | Significado |
| --- | --- |
| Asunto | El asunto completo, por ejemplo `CN=Example Root CA, O=Example, C=XX`. |
| Emisor | El emisor completo, con la misma forma que el asunto. |
| Número de serie | El número de serie, en hexadecimal en minúsculas, con dos puntos entre cada byte. |
| Contenido PEM | El texto del certificado en formato PEM. |

## Qué hace la importación {#what-the-import-does}

1. Haga clic en **Importar certificados** o **Importar**. Se abre el cajón **Importar autoridades de certificación**.
2. Seleccione el archivo PEM. El portal lee el archivo en el navegador. Muestra **Archivo cargado** con el tamaño del texto. Si no se puede leer el archivo, el portal muestra **Error al leer el archivo.**
3. Haga clic en **Importar**. El portal envía el texto del archivo al servidor.
4. El servidor detiene la importación si la política de certificados digitales de votantes no está **Habilitado**.
5. El servidor busca cada bloque de certificado en el texto. Si no hay ningún bloque, el servidor detiene la importación.
6. Para cada bloque, en el orden del archivo, el servidor:
   - lee el certificado. Si no puede leerlo, registra un error para él;
   - comprueba la fecha de caducidad. Si el certificado está caducado, registra un error para él;
   - comprueba la huella. Si el certificado ya está en el proceso electoral, lo cuenta como omitido;
   - guarda el certificado en el proceso electoral.
7. El servidor escribe una entrada en el registro electoral para los certificados que guardó. La entrada indica los sujetos de los certificados guardados.
8. El portal muestra el resultado y cierra el cajón. Las notificaciones aparecen en [Mensajes y errores](#messages-and-errors).

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde | Significado y acción |
| --- | --- | --- |
| Se importaron \{number\} certificado(s). | Notificación | El número de certificados que el servidor guardó. |
| \{number\} certificado(s) omitido(s) (ya presentes). | Notificación, estilo de error | Estos certificados ya están en el proceso electoral. No hace falta ninguna acción. |
| Problemas en la importación: \{errors\} | Notificación, estilo de error | Cada error indica el número del bloque en el archivo. Lea el error, corrija el bloque e importe de nuevo. |
| `Certificate <n>: expired on <YYYY-MM-DD>` | Parte de **Problemas en la importación** | El certificado está caducado. No se importa. |
| `Certificate <n>: Failed to parse PEM: ...` | Parte de **Problemas en la importación** | El bloque no es un PEM válido. Compruebe las líneas del bloque. |
| `Certificate <n>: Failed to parse X.509 certificate: ...` | Parte de **Problemas en la importación** | El bloque es un PEM válido, pero no es un certificado X.509. |
| Error al importar: \{error\} | Notificación | La petición falló. El cajón permanece abierto. Lea el mensaje. |
| Error al leer el archivo. | Cajón | El navegador no pudo leer el archivo. Seleccione el archivo de nuevo. |
| `No valid PEM certificates found in input` | Error del servidor | El archivo no tiene ningún bloque entre la línea de inicio y la línea de fin. Compruebe el archivo. |
| `Digital certificate authentication is not enabled for this election event` | Error del servidor | Establezca **Voter Digital Certificate Policy** en **Habilitado** y, después, importe de nuevo. |

## Páginas relacionadas {#related-pages}

- [Certificados](../02-election-event/15-election_management_election-event_certificates.md): la pestaña **Certificados** del proceso electoral.
- [Formatos de archivo de importación](index.md): todos los archivos de importación.
