---
id: election_management_election_event_data
title: Datos
description: "La pestaña Datos es similar en varias entidades del sistema (procesos electorales, elecciones, preguntas y candidatos). En esta pestaña puede configurar los valores principales de cada entidad."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


La pestaña Datos es similar en varias entidades del sistema (procesos electorales, elecciones, preguntas y candidatos). En esta pestaña puede configurar los valores principales de cada entidad. En concreto, para los procesos electorales, aquí se gestionan todos los datos relacionados.


## Botones de acción de la pestaña Datos del proceso electoral {#actions-buttons-in-the-election-event-data-tab}

- **Exportar**: Exporta los datos del proceso electoral.
- **Importar candidatos**: Importa los candidatos del proceso electoral desde un archivo. El archivo debe tener el diseño fijo de una integración externa concreta. No es una importación de uso general. Para añadir candidatos, use el árbol del proceso electoral o importe un proceso electoral.
- **Google Meet**: Genera un enlace de Google Meet y crea el evento en Google Calendar.


## Secciones de la pestaña Datos del proceso electoral {#sections-in-the-election-event-data-tab}

Cada sección tiene una finalidad específica y ofrece un desglose completo de la información:

- **General**: Incluye los datos básicos.
- **Fechas**: Fechas de inicio y de fin del proceso electoral.
- **Idioma**: Idiomas admitidos para este proceso electoral.
- **Diseño de la papeleta**: Funciones personalizadas de la papeleta, incluidos el diseño, los logotipos, los enlaces y más.
- **Canales de voto permitidos**: Métodos de votación aplicables.
- **Prefijos personalizados de URL**: Defina URL personalizadas para los portales de votación e inscripción y para el punto final SAML.
- **Materiales de apoyo**: Documentos disponibles en el portal de votación para que los votantes los consulten.
- **Configuración avanzada**: Activa el bloqueo del sistema, el tiempo de espera de la sesión del portal de votación y el cierre de sesión forzado.

A continuación se describe cada sección con detalle.

---

## General {#general}

Configure los datos básicos y los nombres multilingües de su proceso electoral.

- **Pestañas de idiomas**: Configure cómo aparece su proceso electoral en distintos idiomas en el portal de votación.
- **Nombre**: Escriba el nombre oficial de su proceso electoral.
- **Alias (opcional)**: Alias interno que se usa solo en el menú lateral del sistema.
- **Descripción (opcional)**: Escriba una descripción de su proceso electoral.

## Idioma {#language}

Gestione las opciones de idioma de su proceso electoral. Los idiomas seleccionados estarán disponibles para las elecciones de este proceso electoral.

- Use los botones de opción para seleccionar los idiomas disponibles.
- Establezca el idioma predeterminado seleccionando **Por defecto** junto al idioma correspondiente.
- **Política de detección de idioma**: 
  Afecta al idioma predeterminado del portal de votación.
  - **Detectar desde el navegador**: El idioma predeterminado lo determina el navegador.
  - **Forzar predeterminado**: El idioma predeterminado será el que esté seleccionado como **Por defecto**.

## Diseño de la papeleta {#ballot-design}

Gestione la forma en que la papeleta aparece en el portal de votación.

- **Desactivar soporte de auditoría de la papeleta**: Activa o desactiva la posibilidad de que los votantes verifiquen el cifrado de la papeleta.
- **Saltar pantalla para escoger elección**: Omite la selección de elección en el portal.
- **Mostrar perfil de usuario**: Muestra la información del perfil del usuario en el portal de votación.
- **Mostrar logs de votación**: Política para activar los registros inmutables de voto emitido en el localizador de papeletas.
- **Logo URL (opcional)**: Proporcione un enlace a un logotipo que se mostrará.
- **URL de redirección al finalizar (opcional)**: Redirige a los usuarios a una URL al terminar de votar.
- **CSS personalizado**: Aplica estilos personalizados al diseño de la papeleta. Los mensajes de error y de aviso de la papeleta exponen clases CSS estables a las que se puede apuntar aquí; consulte [Dar estilo a los errores y avisos de la papeleta con CSS personalizado](../08-ballot-errors-custom-css.md).

## Canales de voto permitidos {#voting-channels-allowed}

Defina los métodos de votación disponibles para este proceso electoral.

- Use los botones de opción para habilitar los canales de voto aplicables.
- **En línea**: Canal principal de votación remota. Al iniciar o cerrar el canal En línea también se rige el ciclo de vida de la votación anticipada (véase más abajo).
- **Kiosco**: Votación presencial mediante un dispositivo. Su estado es independiente de los demás.
- **EARLY_VOTING**: Habilita un período de votación anticipada antes de la ventana de votación en línea para los votantes cuyas áreas tienen activada la votación anticipada.
  - Solo aparece si se permite aquí; cuando se inicia en la pestaña Publicar, solo pueden votar los votantes asignados a áreas con la política de votación anticipada habilitada.
  - El canal En línea rige la votación anticipada:
    - Cuando se inicia o se cierra En línea, la votación anticipada se cierra automáticamente si estaba habilitada.
    - La votación anticipada no puede iniciarse una vez iniciada la votación en línea (ni después).
  - Si un canal que ya está iniciado se deshabilita después manualmente en esta sección, no se toma ninguna medida inmediata; los botones de acción permanecen deshabilitados hasta que el canal se vuelva a permitir.

## Prefijos personalizados de URL {#custom-urls-prefix}

Cree prefijos de URL personalizados para los portales de votación e inscripción y para el punto final SAML.

- Escriba el prefijo deseado para cada punto final.

**Ejemplos:**

- Escriba "myelection" en **Inicio de sesión**:  
  la URL pasa a ser `https://myelection.sequent.vote`
- Escriba "enrollment" en **Inscripción**:  
  la URL pasa a ser `https://enrollment.sequent.vote`

## Materiales de apoyo {#support-materials}

Proporcione documentos a los que los votantes pueden acceder en el portal de votación.

- **Materiales de Soporte activados**: Habilita o deshabilita documentos de apoyo adicionales.
- **Añadir**: Adjunte documentos con los siguientes campos:
  - Título
  - Subtítulo
  - **Oculto**: Controla la visibilidad en el portal
  - Arrastre y suelte el archivo
  - Guardar

## Configuración avanzada {#advanced-configuration}

Configure comportamientos avanzados del sistema para este proceso electoral.

- **Política de cifrado de concurso**:
  - **Concurso único**: Cifra cada pregunta por separado.
  - **Varios concursos**: Cifra varias preguntas juntas para habilitar la auditoría a nivel de papeleta.
- **Estado de confinamiento**: Cuando está habilitado, no se pueden hacer cambios en este proceso electoral. Esta acción es irreversible.
- **Política de cuenta regresiva del portal de votación**:
  - Defina la duración del tiempo de espera de la sesión en segundos.
  - Configure los umbrales del aviso de cuenta regresiva y de la alerta de cierre de sesión.
- **Política de ceremonias de llaves/recuento**:
  - Permita la generación automática de claves y recuentos, lo que elimina la necesidad de la participación de los fideicomisarios.
- **Política de votación ponderada**:
  - **Votación Ponderada por Áreas**: Habilita la votación ponderada por áreas.
  - **Votación Ponderada por Votantes**: Asigna a cada votante su propio peso, de forma que un votante con peso `w` aporta `w` votos. Añada una columna `vote-weight` al CSV de votantes importado con un número entero entre 1 y 100000. Un votante sin columna, o con una celda vacía, vota con peso 1. Los nombres parecidos que solo difieren en mayúsculas o en `_` y `.` — `vote_weight`, `voteWeight`, `vote.weight` — se rechazan en lugar de importarse, porque se guardarían con un nombre que el recuento no lee. Una columna llamada simplemente `weight` **no** se rechaza: es el nombre del peso de área, que no tiene relación con esto, así que se importa como un atributo normal y todos los votantes cuentan como 1. La columna recibe el nombre del atributo de votante `vote-weight` al que se convierte, de modo que un archivo de votantes exportado puede editarse y volver a importarse sin cambios. No puede combinarse con la Votación Delegada, y todas las preguntas del proceso electoral deben usar el algoritmo de conteo Mayoría Plural (Plurality at Large): contar una papeleta más de una vez no tiene un significado definido para los demás, así que el recuento se niega a ejecutarse. Las actas de escrutinio aprobadas también se rechazan por el mismo motivo: un acta de escrutinio informa un recuento de papeletas en papel y no tiene dónde registrar un peso, así que sus votos se sumarían a los totales ponderados con peso uno cada uno, lo que decide preguntas en lugar de solo subcontarlos. Las papeletas decodificadas tampoco pueden incluirse en los resultados publicados, por la razón de la advertencia siguiente. Compruebe ambas cosas antes de publicar las papeletas: ni el algoritmo de conteo ni una papeleta publicada se pueden cambiar una vez iniciada la votación. Los pesos de todos los que votan en un área de una pregunta también deben sumar no más de 1000000, así que un número reducido de votantes con pesos muy altos se rechaza aunque cada peso sea individualmente admitido. **Esto se comprueba al ejecutar el recuento, que ocurre después de cerrar la votación.** La importación escribe un aviso en el registro del servidor cuando los pesos de un archivo superan ese límite, pero el portal de administración no muestra nada al respecto, y solo cuenta ese archivo, no todo lo que ya hay en el área. Doscientos un votantes con 5000 cada uno superan el límite mientras cada uno de ellos se importa sin problemas. Planifíquelo antes de abrir la votación: la solución posterior es reescalar todos los pesos, lo que cambia el resultado. El límite no viene de la mezcla, que con esta política es como máximo un texto cifrado por votante y por lote sin importar lo grandes que sean los pesos; viene de que el recuento expande los votos de cada lote por el multiplicador de ese lote.

    Los totales de participación con esta política cuentan poder de voto y no votantes: el censo de votantes elegibles y el total de papeletas emitidas son sumas de pesos, así que no coincidirán con un recuento de personas mostrado en otro lugar. Las papeletas sin votante coincidente son la excepción: no tienen votante y, por tanto, ningún peso, y se cuentan una a una, lo que mezcla unidades en el total de papeletas emitidas y en el porcentaje auditable de papeletas. Las cifras de Total de votantes y No votantes de la pantalla de Recuento son poder de voto con etiquetas de recuento de personas.

    Si alguna área todavía tiene un **Peso** de una configuración anterior de Votación Ponderada por Áreas, el recuento se rechaza hasta que se borre, porque las dos ponderaciones se multiplicarían. **Compruébelo antes de publicar las papeletas.** Seleccionar esta política oculta la columna y el campo Peso sin borrar lo que contienen, así que un área puede conservar un peso que ninguna parte de la interfaz muestra, y el rechazo solo llega cuando se crea el recuento, después de cerrar la votación, cuando las papeletas ya no se pueden volver a publicar y no queda ninguna solución. Para borrarlo hay que volver a Votación Ponderada por Áreas, que vuelve a mostrar el campo Peso, borrar el peso de cada área, cambiar a Votación Ponderada por Votantes y, solo entonces, publicar las papeletas.

    :::caution Defina los pesos antes de extraer las papeletas para un recuento
    Extraer las papeletas para un recuento las escribe en el tablón, que solo admite añadidos. Si después se cambia un peso de voto, o si un votante que ya había votado se deshabilita o se elimina, el tablón ya no coincide y el recuento se niega en lugar de publicar un total que cuente a algunos votantes con su peso antiguo y a otros con el nuevo a la vez. Volver a poner exactamente lo que cambió lo corrige. Nada más lo corrige: las papeletas que ya están en el tablón se mezclarán y se publicarán haga lo que haga el recuento, así que los pesos extraídos mal no se pueden corregir después, y un votante cuya cuenta se eliminó en lugar de deshabilitarse no se puede restaurar en absoluto, por lo que esa área no podrá contarse nunca.

    El rechazo no garantiza que los pesos usados sean los configurados ahora. Un peso aumentado de modo que solo añade lotes que la ejecución interrumpida aún no había escrito se aplica tal como se aumentó, porque nada que ya esté en el tablón lo contradice. Establezca bien los pesos antes de iniciar el recuento en lugar de confiar en que esto detecte un cambio.
    :::

    :::danger Los pesos de los votantes son públicos, y los resultados se pueden atribuir
    Un peso de voto se aplica dividiéndolo en potencias de dos. Un área de una pregunta se mezcla como hasta 17 lotes; el lote en la posición `n` cuenta cada papeleta del área `2^n` veces, y la papeleta de un votante se coloca en el lote de cada potencia de dos que sumadas dan su peso: el peso 21 va a los lotes de 1, 4 y 16. Ningún lote contiene nunca la misma papeleta dos veces, así que nada del tablón se repite de forma que revele un peso.

    Eso no basta para mantener privado un peso. El tablón es público para que la mezcla pueda verificarse, el mismo texto cifrado aparece en cada lote que selecciona su peso, y cada texto cifrado puede vincularse con el votante que lo emitió, así que sumar los lotes en los que aparece una papeleta recupera exactamente el peso de ese votante. **Por tanto, el peso de un votante es público, y es público quién lo tiene.** Esta es la otra cara de una propiedad que algunos organismos quieren: los pesos publicados se pueden auditar contra el tablón.

    Los totales publicados por candidato son sumas de los pesos de los votantes que eligieron a cada candidato. Cuando los pesos difieren entre sí, esa suma con frecuencia identifica exactamente quién votó a quién; con pesos distintos como 1, 2, 4, 8 siempre lo hace. Desactivar las papeletas decodificadas en los resultados no impide esto; los totales por sí solos bastan. Las respuestas de voto por escrito son todavía peores, porque una candidatura manual poco común aparece con exactamente el peso del votante que la escribió.

    Un lote con muy pocas papeletas publica las opciones de esos votantes al descifrarse, porque una mezcla de uno no oculta nada. Qué lotes son pequeños depende de cómo se reparten los pesos, no de lo grandes que sean: 999 votantes con peso 2 y uno con peso 3 dejan el lote de 1 con una sola papeleta. Al extraer las papeletas se registra un aviso que nombra cualquier lote con menos de cinco. No lo rechaza, porque para entonces la votación ya ha cerrado y no queda ninguna solución.

    Ese aviso es un mínimo, no una garantía. Un votante queda expuesto siempre que los lotes lo aíslen de algún modo —dos lotes cuyos votantes difieren en una sola persona exponen a esa persona sea cual sea el tamaño de ambos lotes—, y nada lo detecta. Compruebe la distribución de los pesos antes de abrir la votación.

    No use esta política cuando se exija secreto de voto. Se adapta a organismos que ya publican cómo votó cada miembro, como algunos votos de accionistas o delegados. Cuando el secreto importa y los votantes se reparten en pocas clases de peso, la Votación Ponderada por Áreas da ponderación sin esta divulgación, porque cada votante de un área comparte un solo peso.
    :::

    :::caution
    El atributo `vote-weight` debe estar declarado en el perfil de usuario de Keycloak del realm del proceso electoral antes de usarlo. Hasta entonces, la columna de peso falta en la exportación de votantes, el campo no aparece al editar un votante, y editar un votante desde el portal de administración borra cualquier peso que tuviera. Importar pesos y contarlos funciona igualmente. Los perfiles de usuario de los realms no se gestionan desde esta aplicación, así que declarar el atributo es un paso del administrador en la configuración del realm.
    :::
  - **Votación Ponderada Deshabilitada**: Desactiva la votación ponderada.
- **Política de votación delegada**:
  - Permite que los votantes deleguen su voto en otro votante. Hay que incluir una columna adicional en el CSV de votantes importado, con el nombre `delegate-vote-to` y el nombre de usuario del votante en quien se delega el voto.
