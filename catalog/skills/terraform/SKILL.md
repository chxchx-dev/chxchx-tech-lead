# Terraform

## Estado

- Estado **remoto con bloqueo** y cifrado (S3 + DynamoDB, GCS, Terraform Cloud); nunca en el repositorio, porque
  contiene secretos. Un estado por entorno y por componente, de tamaño manejable.
- Nadie edita el estado a mano; usa `terraform state` y `moved`/`import` con revisión.

## Código

- Módulos pequeños con entradas y salidas tipadas, descripciones y validaciones (`validation {}`); versiónalos y
  fija las versiones de proveedores y de Terraform (`required_version`, `.terraform.lock.hcl` versionado).
- Entornos diferenciados por variables y estados, no copiando directorios. Etiqueta los recursos (dueño,
  entorno, costo).
- Evita `count` con listas que cambian de orden; usa `for_each` con claves estables.
- `lifecycle { prevent_destroy = true }` en lo que no debe borrarse (bases de datos, almacenamiento de datos).

## Secretos y seguridad

- Sin credenciales en variables ni en el código: identidades de la plataforma (OIDC, roles) y un gestor de
  secretos. Marca las salidas sensibles como `sensitive`.
- Mínimo privilegio para el usuario que aplica; separa quién planifica de quién aplica en producción.
- Analiza con `tflint`, `terraform validate` y escáneres de seguridad (`tfsec`/`checkov`).

## Flujo seguro de cambios

1. `terraform fmt -check` y `terraform validate`.
2. `terraform plan -out=plan.bin` y **revisa el plan completo**: busca destrucciones y reemplazos
   (`-/+`) no esperados, sobre todo en recursos con datos.
3. Aprobación humana del plan para producción; aplica **ese** plan guardado, no uno nuevo.
4. Tras aplicar, verifica el resultado y que un nuevo `plan` no muestre diferencias (deriva cero).

- Detecta la deriva con planes periódicos y trata los cambios manuales como incidencias.
- Antes de operaciones destructivas, comprueba que hay copia de los datos y una vía de reversión.
