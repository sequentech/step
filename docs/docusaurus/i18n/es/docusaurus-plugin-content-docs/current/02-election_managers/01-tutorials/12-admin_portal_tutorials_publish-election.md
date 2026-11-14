---
id: admin_portal_tutorials_publish-election
title: Publicar elección
description: "Cada vez que un administrador modifica datos a nivel de proceso electoral, elección, pregunta o candidato, esos cambios deben publicarse para que sean visibles en el portal de votación."
---
import GoogleVideo from '@site/src/components/GoogleVideo';

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

<GoogleVideo id="1RUe2XgKsC36bZiJ8Io1iAVXYn3KDhBht" />

Cada vez que un administrador modifica datos a nivel de proceso electoral, elección, pregunta o candidato, esos cambios deben publicarse para que sean visibles en el **portal de votación**. Así se garantiza que la interfaz de votación siga siendo exacta y esté sincronizada con la última configuración administrativa.

La vía principal está en [Publicar y gestionar el período de votación](../03-procedures/05-publish.md).

## Paso 1: Abra la pestaña Publicar {#step-1-open-the-publish-tab}

1.  Abra el proceso electoral en la barra lateral.
2.  Haga clic en la pestaña **Publicar**. Cada elección también tiene su propia pestaña **Publicar**, solo para esa elección.
3.  La lista **Historico de Cambios** muestra las publicaciones anteriores. En cada fila, el icono del ojo abre la publicación y el icono de vista previa abre su vista previa. Si todavía no hay ninguna publicación, la pestaña muestra «Ninguna Publicación aún.».

![Pantalla del historial de publicaciones](../../../../../../docs/02-election_managers/01-tutorials/assets/publish_history_log.png)

## Paso 2: Genere la publicación {#step-2-generate-the-publication}

1.  Haga clic en **Generar Publicación**. Si ya existe una publicación, haga clic en **Publicar Cambios**.
2.  Haga clic en **Confirmar**. Si inició sesión hace más de 60 segundos, el portal de administración le pedirá de nuevo su contraseña. Consulte [Acciones sensibles](../00-before-you-start.md#sensitive-actions).

![Confirmación de publicación y contraseña](../../../../../../docs/02-election_managers/01-tutorials/assets/publish_password_auth.png)

3.  Espere al mensaje «Papeleta generada». La página **Cambios a Publicar** compara la publicación **Actual** con los **Cambios a Publicar**. Muestra los datos añadidos en verde y los datos eliminados en rojo.

![Vista de comparación de la publicación](../../../../../../docs/02-election_managers/01-tutorials/assets/publish_diff.png)

## Paso 3: Previsualice la papeleta (opcional, pero recomendado) {#step-3-preview-the-ballot-optional-but-recommended}

Antes de hacer que los cambios estén disponibles para los votantes, puede usar la función **Vista previa** para comprobar el aspecto de la papeleta digital.

1.  En la página **Cambios a Publicar**, haga clic en **Previsualizar**.
2.  En **Selecciona el área para la vista previa**, seleccione un área, ya que las papeletas pueden variar según el área.
3.  Haga clic en **Vista previa** para abrir la papeleta del área en el **portal de votación**, o haga clic en **Copiar enlace** para compartir el enlace.

![Configuración de la vista previa de la papeleta](../../../../../../docs/02-election_managers/01-tutorials/assets/publish_preview_dialog.png)

## Paso 4: Publique los cambios {#step-4-publish-the-changes}

1.  En la página **Cambios a Publicar**, haga clic en **Publicar Cambios**. No aparece ninguna ventana de confirmación.
2.  Espere al mensaje «Papeleta publicada». La lista **Historico de Cambios** muestra la nueva publicación. Los votantes ven la última publicación.

:::tip **Cuándo publicar**
Debe realizar una publicación después de:
* Corregir el nombre de un candidato o añadir una descripción.
* Añadir o quitar una pregunta en un área específica.
* Actualizar el diseño general de la papeleta o las cadenas de localización.
:::
