---
id: admin_portal_reference_user_manual_templates
title: Plantillas
description: "Las plantillas le permiten personalizar los documentos HTML que se usan en informes y comunicaciones. Puede añadir imágenes y enlaces, o modificar el texto para adaptarlo a la imagen de marca y a los requisitos de su organización."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->



## Descripción general {#overview}

Las plantillas le permiten personalizar los documentos HTML que se usan en informes y comunicaciones. Puede añadir imágenes y enlaces, o modificar el texto para adaptarlo a la imagen de marca y a los requisitos de su organización.

## Crear una plantilla {#creating-a-template}

1. En el panel izquierdo del portal de administración, debajo de Procesos Electorales y debajo de **Configuración**, haga clic en **Plantillas**.
2. Escriba un nombre y un alias para la plantilla, y después elija el tipo.
3. En **Elige métodos**, active **Documento** y se mostrará el documento HTML predeterminado. Puede modificar esta plantilla; se renderizará a PDF más adelante.
4. **Opcional:** si el tipo de plantilla se usa para comunicaciones con el usuario, puede modificarla activando **Email** o **SMS**.
5. Después de hacer los cambios, haga clic en **Guardar**.

## Uso {#usage}

El sistema usará automáticamente la plantilla creada al generar documentos de ese tipo (por ejemplo, durante el proceso de recuento o al exportar datos).

También puede usar esta plantilla al [crear informes](../../../01-tutorials/18-reports_and_templates.md) manualmente en la pestaña **Informes**.

## Variables del votante {#voter-variables}

Las comunicaciones y los informes que muestran un votante cada vez, como la carta de información para el votante y el informe de verificación manual, reciben el votante como `user`: `user.first_name`, `user.last_name`, `user.username`, `user.email`, y cada atributo de usuario como `user.<attribute>`. Por ejemplo, una carta puede imprimir `{{user.ward}}` o una dirección postal guardada en atributos personalizados sin configuración adicional. Use los auxiliares `concat`, `format_pattern` y `upper` para imprimir los campos que algunos votantes dejan vacíos. Consulte [Variables del votante](../../02-election-event/13-02-election_management_election-event_templates.md#voter-variables) para ver la lista completa de variables y auxiliares.

## Variables secretas del votante {#secret-voter-variables}

Una comunicación o un informe real por votante puede usar campos configurados con `sequent.secret=true`.
Los campos secretos nunca están disponibles en los informes agregados de elección, participación, recuento, resultados o actividad.

Use las rutas normales de las variables de votante. Para un atributo secreto llamado `customerReference`:

```handlebars
{{user.customerReference}}
{{lookup user.attributes "customerReference"}}
```

La primera expresión devuelve el primer valor. La segunda accede a la matriz completa almacenada para el atributo.

Cada campo secreto usado por una salida también debe declararse de forma explícita. En la configuración guardada de la plantilla de informe, añada `secret_attribute_names` junto a la configuración existente de documento, correo electrónico o SMS:

```json
{
  "secret_attribute_names": ["customerReference"],
  "document": "<p>Reference: {{user.customerReference}}</p>"
}
```

La declaración es una lista de permitidos, no solo documentación. Al generar la salida, Step confirma que cada nombre sigue configurado como secreto y descifra solo los campos declarados. Una búsqueda dinámica de Handlebars no puede acceder a un secreto no declarado.

El editor de comunicaciones de la pestaña **Votantes** crea esta declaración automáticamente a partir de los nombres de atributos secretos configurados que aparecen en el contenido del correo electrónico o del SMS. Las plantillas de informe configuradas mediante JSON o mediante la configuración del proceso electoral deben incluir la declaración por sí mismas.

Ejecutar una salida con una declaración no vacía requiere `voter-secret-attribute-read`, además del permiso normal de envío o de informe. Trate el mensaje o el informe generado como datos sensibles y aplique sus controles normales de acceso a documentos y a la entrega.

Los cuerpos de los correos electrónicos y de los SMS no están cifrados con contraseña: los destinatarios reciben mensajes legibles, incluidos los valores secretos declarados. Los informes en archivo siguen su política de cifrado configurada y los controles privados de acceso a documentos. Los registros de auditoría de entrega no contienen el asunto ni el cuerpo del mensaje generado.

### Entrega solo de prueba con Console {#test-only-console-delivery}

Si establece explícitamente `EMAIL_TRANSPORT_NAME=Console` o `SMS_TRANSPORT_NAME=Console`, el mensaje completo generado se imprime en la consola del worker de Windmill en lugar de enviarse. Esto incluye los atributos secretos descifrados, por lo que puede usarse para inspeccionar un código de inicio de sesión de prueba.

Use únicamente votantes y credenciales sintéticos. No active el transporte Console en producción ni envíe estos registros a un destino sin restricciones. Los transportes de entrega reales no registran el cuerpo de los mensajes; un nombre de transporte desconocido provoca un error en lugar de usar Console en silencio.
