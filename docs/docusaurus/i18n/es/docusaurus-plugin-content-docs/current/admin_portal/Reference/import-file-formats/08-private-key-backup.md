---
id: import_format_private_key_backup
title: Archivo de copia de seguridad de la clave privada
sidebar_position: 9
description: "El archivo de copia de seguridad de la clave privada: su nombre y contenido, cómo lo comprueba el portal de administración, los resultados de la comprobación y cómo mantener el archivo a salvo."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de copia de seguridad de la clave privada contiene el fragmento de clave de un fideicomisario, en forma cifrada. Cada fideicomisario descarga el archivo durante la ceremonia de claves. El fideicomisario sube el mismo archivo de nuevo para comprobarlo. Más tarde, en la ceremonia de recuento, el fideicomisario sube el archivo otra vez para entregar el fragmento de clave a la plataforma. Usted no crea este archivo. Solo lo descarga, lo guarda y lo sube sin ningún cambio.

Para los pasos, consulte [Realizar la ceremonia de claves](../../procedures/04-keys.md) y [Realizar la ceremonia de recuento](../../procedures/06-tally.md).

:::danger ADVERTENCIA: archivo de copia de seguridad de la clave privada
Guarde al menos dos copias del archivo en dos lugares seguros diferentes. El recuento necesita suficientes fideicomisarios para subir un archivo válido. El número de fideicomisarios lo fija el umbral de la ceremonia de claves. Si muy pocos fideicomisarios conservan un archivo válido, los votos no se pueden descifrar. No comparta el archivo con otras personas.
:::

## Qué es el archivo {#what-the-file-is}

| Propiedad | Valor |
| --- | --- |
| Nombre del archivo | `encrypted_private_key_trustee_<user name>_<election event name>.txt` |
| `<user name>` | El nombre de usuario del fideicomisario que descarga el archivo. |
| `<election event name>` | El nombre (alias) del proceso electoral, tal como lo muestra el portal de administración. |
| Tipo | Texto plano, una línea, sin encabezado. |
| Contenido | El fragmento de clave privada cifrado del fideicomisario, escrito en Base64 (alfabeto estándar, sin relleno `=`). |
| Un archivo por | Un fideicomisario y una ceremonia de claves. El archivo de otro fideicomisario no funciona. |

Ejemplo del nombre: `encrypted_private_key_trustee_trustee1_Election Event 1.txt`.

:::note NOTA: archivo de copia de seguridad de la clave privada
El contenido del archivo no es un ejemplo aquí a propósito. Nunca publique un archivo real.
:::

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| No cambie el archivo | La plataforma compara el texto completo del archivo con el texto que guarda. Un carácter distinto, un salto de línea al final o un espacio da el resultado **No válido**. |
| No lo abra en un editor que guarde cambios | El editor puede añadir un salto de línea al final del archivo. |
| Nombre del archivo | Puede cambiar el nombre del archivo. La plataforma no comprueba el nombre. |
| Quién puede subirlo | Solo el fideicomisario propietario del archivo. La comprobación usa el fideicomisario de su sesión. |
| Codificación | Texto plano. El portal de administración lee el archivo como texto. |

## Descarga en la ceremonia de claves {#download-in-the-keys-ceremony}

Un fideicomisario solo puede descargar el archivo cuando se cumplen estas condiciones:

- La ceremonia de claves tiene el estado de ejecución `IN_PROGRESS`.
- El fideicomisario forma parte de la ceremonia de claves.
- El fideicomisario todavía no ha comprobado la clave.

Si el fideicomisario ya ha comprobado la clave, el portal de administración muestra "Tu clave privada ya se había descargado y verificado." Si la ceremonia ha avanzado, muestra "La descarga de la clave privada ya no está disponible porque la ceremonia ha avanzado." Si la plataforma no tiene ninguna clave, muestra "Error de descarga, fichero vacío". Si ocurre otro error, muestra "No se pudo descargar la clave privada. Inténtalo de nuevo."

Cuando el fideicomisario hace clic en **Descargar tu Clave Privada Encriptada**, el estado del fideicomisario cambia a *Fragmento Privado de Clave Descargado*. Aparece el mensaje "Clave Privada Encriptada descargada correctamente." El botón **Siguiente** sigue deshabilitado hasta que la descarga tenga éxito. Después, el fideicomisario debe confirmar dos copias de seguridad: **Primera copia de seguridad realizada** y **Segunda copia de seguridad realizada**.

## Comprobación en la ceremonia de claves {#check-in-the-keys-ceremony}

En el paso **Verifica tus Copias de Seguridad de tu Clave Privada Encriptada**, el fideicomisario sube un archivo de copia de seguridad. Puede intentarlo tantas veces como haga falta, con copias distintas. La comprobación solo funciona mientras la ceremonia de claves tenga el estado de ejecución `IN_PROGRESS` o `SUCCESS`.

| Resultado | Mensaje | Significado |
| --- | --- | --- |
| Válido | "Copia de seguridad verificada correctamente." | El archivo es igual que el archivo que guarda la plataforma. El estado del fideicomisario cambia a *Fragmento Privado de Clave Comprobado*. |
| No válido | "Copa de Seguridad de la Clave Encriptada Privada inválida, por favor inténtalo de nuevo" | El archivo es distinto. Suba otra copia. |

Cuando todos los fideicomisarios hayan comprobado sus archivos, la ceremonia de claves cambia al estado `SUCCESS`.

## Uso en la ceremonia de recuento {#use-in-the-tally-ceremony}

En la ceremonia de recuento, cada fideicomisario entrega su fragmento de clave a la plataforma con el mismo archivo. La página muestra el texto "Por favor, importe su fragmento de clave". Un nuevo fragmento de clave solo se acepta cuando se cumplen las dos condiciones siguientes:

- El estado del fideicomisario en este recuento es `WAITING`.
- El recuento tiene el estado de ejecución `STARTED` o `CONNECTED`.

La plataforma compara el archivo con la clave que guarda. Hay tres resultados:

| Resultado | Mensaje | Significado |
| --- | --- | --- |
| `Restored` | "Copia de seguridad verificada correctamente." | El archivo es correcto. El estado del fideicomisario cambia a `KEY_RESTORED`. |
| `AlreadyRestored` | "Tu clave ya se había restaurado." | El archivo es correcto, y este fideicomisario ya entregó la clave antes. No cambia nada. |
| `Invalid` | "Copa de Seguridad de la Clave Encriptada Privada inválida, por favor inténtalo de nuevo" | El archivo es distinto de la clave que guarda la plataforma. Suba otra copia. |

La plataforma comprueba primero el archivo. Si el archivo es distinto, el resultado es `Invalid`, aunque el recuento no acepte una clave nueva. Si el archivo es correcto, pero el recuento o el fideicomisario tienen otro estado, la subida falla con un error. La página muestra entonces el mismo mensaje que para un archivo no válido. Consulte el estado del recuento para obtener más información.

Cuando el número de fideicomisarios con el estado `KEY_RESTORED` es igual al umbral de la ceremonia de claves o mayor, el recuento cambia al estado `CONNECTED`. La plataforma escribe una entrada en el registro electoral por cada clave restaurada.

## Cómo mantener el archivo a salvo {#how-to-keep-the-file-safe}

1. Descargue el archivo en un dispositivo que solo usted use.
2. Copie el archivo en dos dispositivos seguros diferentes. Por ejemplo, dos memorias USB cifradas.
3. Guarde los dispositivos en dos lugares diferentes.
4. Compruebe cada copia en el paso de comprobación de la ceremonia de claves antes de continuar.
5. No envíe el archivo por correo electrónico ni por una aplicación de chat.
6. No guarde el archivo en una carpeta compartida.

## Páginas relacionadas {#related-pages}

- [Realizar la ceremonia de claves](../../procedures/04-keys.md): descargue y compruebe el archivo.
- [Realizar la ceremonia de recuento](../../procedures/06-tally.md): suba el archivo de nuevo.
- [Claves](../User-Manual/Election-Management/Election-Event/Keys/election_management_election-event_keys.md): la pestaña **Claves**.
- [Formatos de archivos de importación](index.md): todos los archivos de importación.
