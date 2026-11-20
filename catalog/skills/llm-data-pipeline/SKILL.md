# Pipeline de datos para LLM

El modelo es tan bueno, y tan seguro, como sus datos. Cada etapa debe ser determinista, medible y versionada.

## Procedencia y permisos

- Registra de dónde viene cada fuente, bajo qué licencia y con qué restricciones (comercial, atribución,
  datos personales). Excluye lo que no puedas usar y deja constancia de por qué.
- Respeta `robots.txt`, términos de uso y solicitudes de exclusión. No mezcles datos privados con corpus
  que se vayan a compartir.

## Etapas, en este orden

1. **Extracción y normalización:** texto limpio y codificación única (UTF-8, normalización Unicode NFC). Conserva
   un identificador de fuente y de documento en cada registro.
2. **Detección de idioma** y descarte de lo que no corresponda; mide la proporción por idioma resultante.
3. **Filtros de calidad:** longitud mínima y máxima, proporción de símbolos y de líneas repetidas, relación
   alfabética, perplejidad con un modelo pequeño si hace falta. Cada umbral se justifica mirando muestras de
   lo que se descarta y de lo que se conserva, no solo estadísticas.
4. **Deduplicación:** exacta (hash) y aproximada (MinHash/LSH) a nivel de documento, y de líneas o párrafos
   repetidos. Duplicados en el entrenamiento memorizan y sesgan la evaluación.
5. **Datos personales y secretos:** busca correos, teléfonos, claves y tokens; reemplaza o elimina y mide la
   tasa de aciertos con una muestra revisada a mano.
6. **Descontaminación:** elimina del entrenamiento todo lo que se solape con los conjuntos de evaluación
   (n-gramas largos exactos). Hazlo antes de entrenar, no después de ver buenos números.
7. **Mezcla y particionado:** fija las proporciones por fuente y dominio, baraja de forma determinista y
   separa entrenamiento, validación y prueba **por documento o por fuente**, nunca por fragmento.
8. **Tokenización y empaquetado:** tokeniza una vez y guarda fragmentos de tamaño uniforme con un índice; evita
   cruzar documentos sin un token separador.

## Validaciones que deben fallar en voz alta

- Esquema: campos obligatorios, tipos, longitudes y codificación válidos en cada registro.
- Conteos: documentos y tokens por etapa y por fuente; una caída inesperada entre etapas es un error.
- Muestreo: revisa manualmente un número fijo de ejemplos por etapa y guarda el resultado.
- Reproducibilidad: la misma entrada y la misma configuración producen los mismos hashes de salida.
- Fugas: ningún documento de validación o prueba aparece en entrenamiento (comprueba por hash y por
  n-gramas).

## Versionado

Guarda un manifiesto por versión del conjunto: fuentes, licencias, filtros con sus umbrales, conteos, hash
de cada fragmento y versión del código. Los datos nunca se editan en el sitio: se genera una versión nueva.
