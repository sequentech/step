---
id: admin_portal_tutorials_key-ceremony
title: Ceremonia de claves
description: "La ceremonia de claves es un procedimiento de seguridad esencial que garantiza la integridad y el secreto de una elección."
---

import GoogleVideo from '@site/src/components/GoogleVideo';

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

<GoogleVideo id="1F7IVr_xcTzLdUQ3K7CkRtoObeVpakzuV" />

La ceremonia de claves es un procedimiento de seguridad esencial que garantiza la integridad y el secreto de una elección. Genera una clave privada criptográfica fragmentada, de modo que ninguna persona ni organización puede descifrar los votos ni manipular el recuento por sí sola.

## Conceptos básicos: criptografía de umbral {#core-concepts-threshold-cryptography}

La seguridad del sistema de Sequent se basa en el **Descifrado por umbral**.

* **Fragmentación**: La clave privada nunca se genera como un único archivo. Se divide en varios fragmentos, y cada uno se asigna a un **fideicomisario** distinto.
* **El umbral**: Es el número mínimo de fideicomisarios necesario para combinar sus fragmentos y descifrar los resultados.
* **Garantía de seguridad**: Un umbral (por ejemplo, 2 de 3) garantiza que, aunque se pierda o se robe un fragmento, la elección siga siendo segura y recuperable, pero solo si el resto de fideicomisarios coopera.

## Paso 1: Inicio por el administrador {#step-1-administrator-initiation}

El administrador de la plataforma actúa como coordinador de la ceremonia.

1.  Inicie sesión con permisos de **administrador**.
2.  Seleccione el proceso electoral y vaya al menú **Claves**.
3.  Haga clic en **Crear Ceremonia de Claves**. Si el proceso electoral ya tiene una ceremonia de claves, haga clic en **Añadir**.
4.  **Configure el umbral**: Establezca el número mínimo de miembros necesarios para el recuento. Sequent recomienda que el umbral sea inferior al número total de fideicomisarios (por ejemplo, un umbral de 2 para 3 fideicomisarios).
5.  **Asignar fideicomisarios**: En **Autoridades**, seleccione los fideicomisarios de la ceremonia. Seleccione al menos tantos fideicomisarios como el umbral.
6.  En **Elección**, seleccione una elección, o deje el campo vacío para **Todas las Elecciones**.
7.  Haga clic en **Crear Ceremonia de Claves** y, después, en **Sí, Crear Ceremonia de Claves**.

![Panel de configuración de la ceremonia](../../../../../../docs/02-election_managers/01-tutorials/assets/keys_config_panel.png)

## Paso 2: Participación de los fideicomisarios {#step-2-trustee-participation}

Una vez creada la ceremonia, cada fideicomisario asignado debe realizar sus pasos de seguridad individuales.

1.  **Inicio de sesión**: Cada fideicomisario debe iniciar sesión con sus propias credenciales únicas.
2.  **Acceder a las claves**: Vaya al menú **Claves** del proceso electoral. Aparece el mensaje "Ha sido invitado a participar a una Ceremonia de Claves." En la fila de la ceremonia, haga clic en el icono de llave **Participar en la ceremonia de claves**.

![Acciones del fideicomisario con las claves](../../../../../../docs/02-election_managers/01-tutorials/assets/keys_trustee_actions.png)

3.  **Generar y descargar**: Haga clic en **Descargar tu Clave Privada Encriptada** para descargar el fragmento de la clave privada.

![Descarga de la clave del fideicomisario](../../../../../../docs/02-election_managers/01-tutorials/assets/keys_download_step.png)

4.  **Copias de seguridad seguras**: Los fideicomisarios deben confirmar que han guardado el fragmento en al menos dos ubicaciones seguras y distintas, normalmente en dispositivos USB cifrados.

![Copias de seguridad seguras del fideicomisario](../../../../../../docs/02-election_managers/01-tutorials/assets/keys_secure_backups.png)

5.  **Verificación de integridad**: El fideicomisario debe volver a subir el archivo al cuadro **Comprobar** para verificar que la descarga se realizó correctamente y que el archivo es válido.

![Verificación de la copia de seguridad de la clave](../../../../../../docs/02-election_managers/01-tutorials/assets/keys_backup_verification.png)

## Paso 3: Seguimiento y éxito {#step-3-monitoring-and-success}

El administrador puede supervisar la página **Progreso de Ceremonia de Claves** para seguir las tareas completadas. En la pestaña **Claves**, haga clic en el icono de ver de la ceremonia.

![Supervisión de la ceremonia de claves](../../../../../../docs/02-election_managers/01-tutorials/assets/keys_ceremony_status.png)

* **Marcas verdes**: Indican que un fideicomisario ha generado, descargado y verificado su fragmento correctamente.
* **Logs**: La tabla **Logs**, en la parte inferior de la pantalla, registra cada paso de la ceremonia con fines de auditoría.

Cuando todos los fideicomisarios completen sus tareas, el estado cambiará a `SUCCESS`.

El portal de administración no puede cancelar ni reiniciar una ceremonia de claves. Para el procedimiento completo, consulte [Realizar la ceremonia de claves](../03-procedures/04-keys.md).

:::danger **Advertencia sobre datos irrecuperables**
Si demasiados fideicomisarios pierden sus fragmentos (y el total baja del umbral establecido), los resultados de la elección **no podrán ser descifrados por nadie**, incluido el soporte técnico de Sequent. Guardar estos fragmentos de forma segura es la responsabilidad más importante de los fideicomisarios.
:::
