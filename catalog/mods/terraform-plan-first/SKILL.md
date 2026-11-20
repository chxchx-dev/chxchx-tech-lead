Nunca ejecutes `terraform apply` sin un `plan` revisado: muéstralo y señala destrucciones y reemplazos (`-/+`), sobre todo en recursos con datos. Aplica el plan guardado, no uno nuevo, y confirma que un segundo plan no muestra diferencias.

No guardes el estado ni credenciales en el repositorio, no edites el estado a mano y protege lo que no debe borrarse (`prevent_destroy`). Fija las versiones de Terraform y de los proveedores.
