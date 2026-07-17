# SleepGuard

SleepGuard es una aplicación de escritorio multiplataforma (Windows / Linux) que controla el acceso al computador según un horario semanal configurable. Ejecuta apagados automáticos a una hora definida y bloquea el uso del equipo si se enciende antes de la hora de activación establecida para ese día.

Pensada para limitar el tiempo de uso del computador (por ejemplo, horarios de descanso o control parental), SleepGuard corre en segundo plano y aplica el horario de forma automática, sin intervención del usuario.

## Características

- **Horario semanal configurable**: define una hora de apagado y una de activación por defecto, con la posibilidad de sobrescribirlas para días específicos de la semana.
- **Apagado automático**: al llegar la hora configurada, el equipo se apaga sin posibilidad de cancelación ni postergación.
- **Notificación previa**: 15 minutos antes del apagado, el sistema operativo muestra un aviso nativo informativo con la hora exacta.
- **Bloqueo por arranque temprano**: si el equipo se enciende antes de la hora de activación del día, se impide el acceso al escritorio y se vuelve a apagar automáticamente.
- **Configuración avanzada opcional**: pantalla principal simple (hora de apagado, hora de activación y un toggle) y una pantalla avanzada para definir excepciones por día.
- **Toggle de habilitación**: permite suspender temporalmente el servicio sin perder la configuración guardada.
- **Multiplataforma**: misma base de código para Windows 10/11 y Linux (Ubuntu 22.04+ / Debian 11+), con inicio automático al arrancar el sistema.
- **Configuración persistente**: los horarios y preferencias se guardan localmente y sobreviven reinicios del sistema.

## Stack técnico

- [Tauri](https://tauri.app/) 2 + Rust — backend nativo, ejecución de comandos del sistema y persistencia de configuración.
- [React](https://react.dev/) 19 + TypeScript — interfaz de usuario.
- [Vite](https://vitejs.dev/) — bundler del frontend.

## Instalación

### Descargar la aplicación (recomendado)

Los instaladores para Windows y Linux están disponibles en la sección de **Releases** del repositorio:

**[Descargar la última versión →](<!-- TODO: enlace a la página de releases -->)**

Descarga el instalador correspondiente a tu sistema operativo y ejecútalo siguiendo las instrucciones en pantalla.

### Compilar desde el código fuente

Requisitos previos:

- [Node.js](https://nodejs.org/) 18 o superior
- [Rust](https://www.rust-lang.org/tools/install) (toolchain estable)
- Dependencias del sistema para Tauri ([guía oficial](https://tauri.app/start/prerequisites/))

Pasos:

```bash
# 1. Clonar el repositorio
git clone <URL_DEL_REPOSITORIO>
cd SleepGuard/SleepGuard

# 2. Instalar dependencias del frontend
npm install

# 3. Ejecutar en modo desarrollo
npm run tauri dev

# 4. Generar el instalador para tu plataforma
npm run tauri build
```

El instalador generado quedará disponible en `src-tauri/target/release/bundle/`.

## Configuración

Al primer inicio, la aplicación detecta el sistema operativo y solicita confirmación antes de continuar a la configuración de horarios. Desde la pantalla principal puedes definir:

- **Hora de apagado (default)**: hora a la que el equipo se apagará todos los días.
- **Hora de activación (default)**: hora antes de la cual el equipo permanecerá bloqueado.
- **Toggle de habilitación**: activa o desactiva el servicio sin perder la configuración.

Desde **Configuración avanzada** puedes definir horarios distintos para días específicos de la semana, dejando el resto con el valor por defecto.

La configuración se almacena localmente en:

- Windows: `%APPDATA%\SleepGuard\`
- Linux: `~/.config/sleepguard/`

## Licencia

<!-- TODO: especificar licencia del proyecto -->
