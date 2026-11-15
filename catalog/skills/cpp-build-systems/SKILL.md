# Build y empaquetado de C++

- Conserva el sistema de build adoptado por el repositorio. Si usa CMake, organiza dependencias, includes, definiciones y opciones por target; evita configuración global que afecte targets ajenos.
- Declara una versión mínima de CMake y de Qt compatible con las plataformas soportadas. Usa componentes Qt explícitos y las funciones de CMake de Qt para MOC, UIC, RCC y recursos.
- No fijes rutas del SDK, compilador o bibliotecas del equipo local. Usa targets importados, presets y toolchains adecuados para cada plataforma.
- Aplica flags según compilador y configuración mediante expresiones de generador; no pases flags de GCC como si fueran portables a MSVC o AppleClang.
- Fija o documenta versiones y procedencia de dependencias. Evita descargas no verificadas durante la configuración y no guardes credenciales en presets o workflows.
- Separa configuración de build, instalación y empaquetado. Comprueba que los artefactos incluyan los recursos, plugins y bibliotecas dinámicas que necesita cada plataforma.
- Conserva comandos reproducibles para configurar, compilar, ejecutar comprobaciones y producir cada paquete desde CI y desde una máquina limpia.
