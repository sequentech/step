---
id: reports_and_templates
title: Informes y plantillas
description: "Para generar un informe, debe seleccionar el proceso electoral en el portal de administración y añadir una entrada para un tipo de informe específico."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Descripción general {#overview}

Para generar un informe, debe seleccionar el proceso electoral en el portal de administración y añadir una entrada para un tipo de informe específico.

## Crear un informe {#creating-a-report}

1. Vaya a la pestaña **Informe** y haga clic en el botón **Crear Informe** o **Añadir**.
2. Seleccione el tipo (consulte las notas especiales de algunos tipos más abajo).
3. **Opcional:** seleccione una **Elección** si el informe es solo de una elección.
4. **Opcional:** elija una plantilla. Debe haberla creado antes (consulte [más información sobre cómo crear plantillas](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md)). Si no selecciona ninguna, se usará la predeterminada del sistema.
5. Haga clic en **Guardar**.

## Generar informes {#generating-reports}

Después de crear una entrada de informe, la verá en la lista. Haga clic en el menú de acciones (tres puntos a la derecha) y elija:

- **Generar:** crea un informe con datos reales.
- **Previsualizar:** genera un ejemplo de esa plantilla de informe con datos de prueba.

## Tipos de informe {#report-types}

### Participación de votantes {#voters-turnout}

**Nota:** Para mostrar los porcentajes de hombres y mujeres, se debe haber añadido el atributo de sexo. Consulte [cómo añadir el atributo de sexo](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

### Otros tipos de informe {#other-report-types}

*Los tipos de informe adicionales se documentarán aquí a medida que estén disponibles.*

## Campos secretos del votante en los informes {#secret-voter-fields-in-reports}

Los campos secretos del votante solo están disponibles para los informes que muestran un votante identificado cada vez. Los ejemplos actuales son las cartas de información al votante y los informes de verificación manual. Los informes agregados, como los de participación, actividad electoral, recuento y resultados, no reciben un objeto de votante y no pueden usar campos secretos.

La plantilla asignada debe hacer referencia a cada campo secreto que usa y declararlo. Por ejemplo:

```json
{
  "secret_attribute_names": ["customerReference"],
  "document": "<p>Reference: {{user.customerReference}}</p>"
}
```

Durante la generación, Step valida la declaración contra el perfil de usuario de Keycloak actual del proceso electoral. Después, descifra solo los nombres declarados para el votante del informe. El operador necesita `voter-secret-attribute-read` además del permiso normal de informes. Un secreto no declarado, eliminado o ilegible hace que la generación falle, en lugar de exponer su representación almacenada.

Consulte [Variables secretas del votante](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables) para ver la forma de las variables y las reglas de declaración.

## Descargar informes con secretos o cifrados {#downloading-secret-bearing-or-encrypted-reports}

Los archivos con secretos usan almacenamiento privado. Para descargar uno, se necesita `voter-secret-attribute-read` además de los permisos ordinarios de documentos, aunque lo haya generado otro administrador.

Clasificar un campo como secreto y cifrar con contraseña son cosas separadas: declarar un secreto no obliga a cifrar todos los informes. Los informes conservan su política de cifrado configurada. Las cartas de información al votante conservan su comportamiento existente de PDF protegido con contraseña.

Para un informe cifrado con contraseña recién generado, descárguelo desde **Informe** o desde la entrada completada en **Tareas**. El portal de administración abre el cuadro de diálogo de contraseña del informe. Para obtener la contraseña se necesita `document-password-read` y, en el caso de un documento con secretos, `voter-secret-attribute-read`. La contraseña pertenece a ese documento generado, así que cambiar después la configuración del informe no cambia la contraseña de un archivo anterior. Los archivos cifrados antiguos que no tienen una contraseña guardada siguen mostrando instrucciones de descifrado. Use la contraseña que se proporcionó al generarlos.

El correo electrónico y los SMS son mensajes de entrega, no archivos de informe cifrados con contraseña. Su contenido sigue siendo legible para el destinatario, incluidos los campos secretos declarados. Consulte [el envío de prueba solo a la consola](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#test-only-console-delivery) antes de probar una plantilla que contenga credenciales.
