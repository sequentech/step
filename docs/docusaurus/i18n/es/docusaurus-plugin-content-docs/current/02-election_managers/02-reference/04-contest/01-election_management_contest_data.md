---
id: election_management_contest_data
title: Datos
description: "La pestaña Datos de la pregunta permite a los administradores gestionar los ajustes principales y las configuraciones de una pregunta concreta dentro de una elección."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-->


# Pestaña Datos de la pregunta

La pestaña **Datos** de la pregunta permite a los administradores gestionar los ajustes principales y las configuraciones de una pregunta concreta dentro de una elección. Muchos elementos son iguales a los de la pestaña **Datos** del proceso electoral, adaptados al contexto de la pregunta. A continuación se indican las secciones principales y sus detalles.

## Resumen de secciones {#sections-overview}

- **General**: datos básicos y nombres multilingües.
- **Sistema de votación de papeletas**: configuración de los distintos sistemas de votación (en desarrollo).
- **Diseño de la Papeleta**: ajustes de visualización y de comportamiento de la papeleta.
- **Nombre de la página**: controla la agrupación de las preguntas en las páginas de votación.
- **Imagen**: suba una imagen o un icono para esta pregunta.
- **Configuración Avanzada**: importe archivos de configuración personalizados para un comportamiento especializado.

---

## General {#general}

Configure los datos básicos y los nombres multilingües de esta pregunta.

- **Nombre de la pregunta**  
  - Escriba el nombre oficial.  
  - Admite varios idiomas si la elección es multilingüe.
- **Alias (opcional)**  
  - Alias solo para el sistema, para mostrar en los menús laterales o en referencias internas.
- **Descripción (Opcional)**  
  - Breve descripción o instrucciones para los administradores. Puede aparecer en información emergente o en textos de ayuda, si se implementa.
- **Pestañas de idiomas**  
  - Si la elección admite varios idiomas, configure cómo aparecen el nombre y la descripción de esta pregunta en cada idioma.

---

## Sistema de votación de papeletas {#ballot-voting-system}

La configuración del sistema de votación afecta al recuento y a la interfaz del portal de votación (por ejemplo, si el sistema es preferencial o no).

### Algoritmo de conteo {#counting-algorithm}

- **Mayoría Plural**
  - Los votantes seleccionan hasta un número determinado de candidatos. Los candidatos con más votos ganan.
  - Es lo más adecuado para elecciones no preferenciales, en las que los votantes eligen sus opciones principales sin ordenarlas.
- **Segunda Vuelta Instantánea (IRV)**
  - Sistema de votación preferencial en el que los votantes ordenan a los candidatos según sus preferencias.
  - Los candidatos con menos votos de primera preferencia se eliminan y sus votos se reasignan hasta determinar los ganadores.
  - Al seleccionar IRV, establezca el parámetro **Votos Máximos** (en **Diseño de la Papeleta**) al menos igual al número de candidatos disponibles para ordenar.

---

## Diseño de la Papeleta {#ballot-design}

Gestione cómo se muestra la papeleta de esta pregunta y cómo interactúan los votantes con ella.

### Ajustes de visualización y alertas {#display-settings--alerts}

- **Resuelto por aclamación**
  - Marca una pregunta cuyos candidatos han sido elegidos sin votación.
  - Las pantallas de votación y de revisión muestran la pregunta, su aviso de aclamación y todos los candidatos disponibles. Sus opciones están deshabilitadas y no se pueden seleccionar.
  - Un aviso traducido específico de la pregunta puede indicarse en `presentation.i18n.<language>.acclamation_description`. Si no se indica, se usa el aviso traducido estándar.
  - Las reglas de selección mínima y máxima no se aplican. Las opciones explícitas de voto en blanco y de voto inválido, así como los votos por escrito, se ignoran en esta pregunta.
  - La pregunta no se codifica en la papeleta. En una elección mixta, el votante emite normalmente las demás preguntas. Si todas las preguntas están resueltas por aclamación, no se crea ninguna papeleta ni ningún ID de papeleta.
  - Los resultados del recuento contienen cero votos, indican como ganadores a todos los candidatos elegibles configurados y sustituyen las cifras de participación por un aviso de aclamación. No se pueden añadir hojas de recuento a una pregunta resuelta por aclamación.
  - Establezca este valor antes de publicar las papeletas y no lo cambie después, porque determina la estructura de codificación de la papeleta.
  - Consulte [Preguntas por aclamación](../11-acclaimed-contests.md) para ver el comportamiento completo para el votante, el verificador, el recuento y los resultados.
- **Votos Mínimos**
  - El número mínimo de selecciones que debe hacer un votante para que la papeleta se considere válida.
  - Si se establece en 0, los votantes pueden enviar la papeleta sin seleccionar ningún candidato.
- **Votos Máximos**
  - El significado depende del algoritmo de conteo:
    - **No preferencial (Mayoría Plural)**: el número máximo de candidatos que puede seleccionar un votante.
    - **Preferencial (Segunda Vuelta Instantánea / IRV)**: la posición de ranking más alta disponible para los votantes. Por ejemplo, un valor de `5` significa que los votantes pueden asignar los rangos del 1 al 5. Debe ser al menos igual al número de candidatos disponibles para ordenar.
  - Funciona junto con la **Política de voto excesivo** para determinar qué ocurre cuando un votante supera este límite.
- **Alerta de subvoto**
  - Muestra una advertencia si el votante selecciona menos opciones que el máximo requerido.
- **Alerta de sobrevoto**
  - Muestra una advertencia si el votante selecciona más opciones de las permitidas.
- **Número de ganadores**
  - Especifique cuántos candidatos pueden ganar esta pregunta.
- **Orden de los candidatos**
  - Determine el orden de visualización:
    - Aleatorio
    - Alfabético
    - Personalizado (al seleccionarlo, active la reordenación manual).

### Editar listas y tipos de selección {#edit-lists--selection-types}

- **Habilitar listas seleccionables**  
  - Permite agrupar candidatos en listas o en selecciones combinadas de lista y candidato.  
  - Cuando está habilitado, configure el comportamiento de votación basado en listas.
- **Máximo de selecciones por tipo**  
  - Defina el máximo de selecciones para cada tipo de candidato o lista.  
  - Ejemplo: máximo de 2 candidatos individuales y 1 selección de lista.

### Políticas {#policies}

Defina las reglas de comportamiento del votante y las respuestas del sistema en distintos escenarios de votación. Para cada tipo de política indicado a continuación, elija una de las opciones:

- **Permitido**: el votante puede enviar la papeleta sin advertencia.
- **Advertir**: el votante puede enviar la papeleta, pero recibe una o más advertencias.
- **Advertir en Revisión** (si está disponible): la advertencia aparece solo durante la fase de revisión.
- **Advertir y Alertar**: la advertencia aparece y un cuadro de confirmación obliga a confirmarla.
- **No permitido**: el votante no puede enviar la papeleta. Puede incluirse una advertencia o alerta antes de bloquearla.

#### Política de Subvoto {#under-vote-policy}

Cuando el votante selecciona menos opciones que el máximo permitido:

- **Permitido**: envía sin advertencia.
- **Advertir**: advertencia durante las fases de papeleta y de revisión.
- **Advertir en Revisión**: advertencia solo en la fase de revisión.
- **Advertir y Alertar**: advertencia durante la papeleta. Se requiere confirmación para continuar.
- **Advertir y Confirmar en Revisión**: advertencia solo en la fase de revisión. Cuando el votante emite la papeleta, un cuadro de diálogo enumera cada pregunta con esta política que tiene posiciones sin completar, con el número de opciones seleccionadas sobre el máximo. El votante puede volver a la revisión para editar las selecciones, o continuar y emitir la papeleta tal como está.

Con **Advertir y Confirmar en Revisión**, el subvoto sigue siendo un voto válido: nunca se obliga al votante a completar una posición. La confirmación se pide cada vez que el votante emite la papeleta, por lo que se vuelve a pedir tras editar la papeleta si aún quedan posiciones sin completar. Una pregunta que se deja vacía también aparece en la lista, siempre que su Política de voto en blanco lo permita. Cuando se muestra el cuadro de diálogo, sustituye al cuadro de confirmación de emisión de la elección, de modo que el votante confirma una sola vez.

#### Política de voto excesivo {#over-vote-policy}

Cuando el votante selecciona más opciones de las permitidas:

- **Permitido**: envía sin advertencia.
- **Advertir**: advertencia durante las fases de papeleta y de revisión.
- **Advertir y Alertar**: advertencia durante la papeleta. Se requiere confirmación para continuar.
- **No permitido con un mensaje de warning y alerta**: el votante no puede enviar la papeleta y aparece una advertencia.
- **No permitido con un mensaje de warning y dehabilitar selecciones adicionales**: aparece una advertencia y se impide hacer selecciones adicionales.

#### Política de Voto Inválido {#invalid-vote-policy}

Cuando la selección del votante no es válida (por ejemplo, un voto nulo o demasiadas selecciones):

- **Permitido**: envía sin advertencia.
- **Advertir**: advertencia durante las fases de papeleta y de revisión.
- **Advertir Inválidos Implícitos y Explícitos**: advertencia por los votos explícitamente inválidos (nulos o anulados) y por los implícitamente inválidos (por ejemplo, demasiadas selecciones).
- **No permitido**: no puede enviar si selecciona opciones no válidas.
- **Permitido con Voto Inválido Exclusivo**: igual que **Permitido**, salvo que seleccionar la opción de voto explícitamente inválido excluye cualquier otra selección. Al elegirla, se borran los demás candidatos seleccionados, y al elegir un candidato, se borra la opción de voto inválido. Así funciona el voto en blanco. Combinar una selección explícitamente inválida con otros candidatos (por ejemplo, desde una aplicación anterior) se cuenta igual que en **Permitido**, porque esa forma de papeleta sigue siendo compatible de forma intencionada para las aplicaciones que dependen de ella.

##### Sistemas preferenciales (p. ej. Segunda Vuelta Instantánea) {#preferential-systems-eg-instant-runoff}

###### ¿Qué se considera un error implícito inválido? {#what-is-considered-implicit-invalid-error}

- **Posición duplicada**: que se permita emitir el voto depende de la política. En cualquiera de los dos casos, asignar la misma posición a 2 candidatos invalida el voto.

###### ¿Qué causa una alerta pero el voto sigue siendo válido? {#what-will-cause-an-alert-but-its-still-valid}

- **Orden de preferencias con huecos**: falta una de las posiciones intermedias, por ejemplo 1, 2, 4, 5.

#### Política de voto en blanco {#blank-vote-policy}

Cuando el votante selecciona la opción de voto en blanco:

- **Permitido**: envía sin advertencia.
- **Advertir**: advertencia durante las fases de papeleta y de revisión.
- **Advertir en Revisión**: advertencia solo en la fase de revisión.
- **No permitido**: no puede enviar una papeleta en blanco.

#### Voto inválido - Política de rango duplicado {#invalid-vote---duplicate-rank-policy}

_Se aplica solo a la votación preferencial (por ejemplo, Segunda Vuelta Instantánea)._

Cuando un votante asigna la misma posición de ranking a dos o más candidatos, el voto se considera inválido. Esta política controla la respuesta de la interfaz:

- **Mostrar advertencia y diálogo (el votante puede continuar)**: se muestra un cuadro de diálogo de advertencia, pero el votante puede enviar la papeleta tal como está.
- **Mostrar advertencia y diálogo (el votante no puede continuar)**: se muestra un cuadro de diálogo de advertencia y el votante no puede enviar la papeleta hasta que resuelva el rango duplicado.

#### Voto inválido - Política de rangos omitidos {#invalid-vote---skipped-ranks-policy}

_Se aplica solo a la votación preferencial (por ejemplo, Segunda Vuelta Instantánea)._

Cuando el ranking de un votante tiene huecos (por ejemplo, posiciones 1, 2, 4 y 5, saltando la 3), esta política controla la respuesta de la interfaz:

- **Mostrar advertencia y diálogo (el votante puede continuar)**: se muestra un cuadro de diálogo de advertencia, pero el votante puede enviar la papeleta con el hueco.
- **Mostrar advertencia y diálogo (el votante no puede continuar)**: se muestra un cuadro de diálogo de advertencia y el votante no puede enviar la papeleta hasta que se rellene el hueco.

### Nombre de la página {#page-name}

- **Definir nombre de la página**  
  - Las preguntas que comparten el mismo nombre de página se agrupan y se muestran en la misma página de votación.  
  - Las preguntas sin nombre de página aparecen en su propia página por defecto.
- **Uso**  
  - Ayuda a organizar las papeletas con varias preguntas, para reducir los pasos de navegación del votante.

---

## Imagen {#image}

Suba una imagen o un icono que represente esta pregunta (por ejemplo, un logotipo o un símbolo).

- **Subir**  
  - Arrastre y suelte un archivo PNG (u otro formato compatible).
- **Vista previa**  
  - El sistema muestra una vista previa después de subir el archivo.
- **Uso**  
  - Puede aparecer en la interfaz de la papeleta para el votante o en las pantallas de administración.

---

## Configuración Avanzada {#advanced-configuration}

Importe o suba un archivo de configuración personalizado para un comportamiento especializado de la pregunta.

- **Subir configuración**  
  - Arrastre y suelte un archivo de configuración (JSON/YAML o un formato propio del sistema).  
  - Aplica ajustes como reglas de validación personalizadas, metadatos ampliados o integraciones.
- **Ejemplos de uso**  
  - Comportamiento de papeleta personalizado que no cubren las opciones por defecto.  
  - Integración con sistemas externos o con herramientas de auditoría especializadas.

---

## Notas y consejos {#notes--tips}

- Muchos ajustes replican los de la pestaña **Datos** del proceso electoral. Aplique flujos de trabajo similares cuando sea posible.  
- Los cambios surten efecto de inmediato o en la siguiente publicación, según el diseño del sistema.  
- Valide las combinaciones de políticas para evitar reglas contradictorias (por ejemplo, una política no puede ser a la vez "Permitido" y "No permitido").  
- Active **Resuelto por aclamación** solo cuando se haya determinado el resultado de la pregunta, para evitar cerrar la votación por accidente.  
- En las elecciones multilingües, verifique las traducciones de los nombres de las preguntas y de las alertas.

---
