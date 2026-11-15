Activa nullable reference types y trata las advertencias como errores en código nuevo. Usa `async/await` de punta a punta (sin `.Result` ni `.Wait()`), pasa `CancellationToken` en operaciones de larga duración y libera recursos con `using`.

Registra dependencias en el contenedor de inyección y programa contra interfaces estrechas. Valida entradas en el borde, no registres datos sensibles y mantén la configuración fuera del código. Cubre la lógica con pruebas (xUnit) y ejecuta `dotnet build` y `dotnet test` antes de cerrar.
