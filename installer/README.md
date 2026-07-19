# SleepGuard — Instalación y autostart (v1.1.1)

Scripts explícitos y auditables para RF-07, RNF-01 y RNF-05.

## Dos capas

| Capa | Qué hace | Cómo se registra |
|------|----------|------------------|
| **Monitor** | Apagado programado + notificaciones | Automático desde la app al activar el servicio (sin admin): systemd user / Task Scheduler |
| **Guard (arranque)** | Si enciendes antes de la hora de activación → bloqueo / apagado | Una vez, con privilegios: botón **Instalar agente de arranque** en la app, o scripts abajo |

## Requisitos previos (desarrollo)

```bash
cd src-tauri
cargo build --release -p sleepguard-core
cargo build --release -p sleepguard-guard
cargo build --release -p sleepguard-app
```

Para empaquetar en Linux, copia el companion al directorio del instalador (también lo hace el workflow de release):

```bash
cp src-tauri/target/release/sleepguard-guard installer/linux/sleepguard-guard
chmod +x installer/linux/sleepguard-guard
```

Ese archivo se incluye en el bundle vía `bundle.resources` y `install-guard.sh` lo usa desde `SCRIPT_DIR`.

## Linux

```bash
# Solo agente de arranque (recomendado si el monitor ya lo registró la app)
sudo ./installer/linux/install-guard.sh --guard-only

# Guard + unit de monitor
sudo ./installer/linux/install-guard.sh
```

Variables opcionales: `GUARD_SRC`, `APP_BIN` (rutas a binarios).

Instala:

1. `/usr/local/bin/sleepguard-guard` — unidad systemd de sistema (`sleepguard-guard.service`) **antes** del display manager.
2. (Sin `--guard-only`) user unit `sleepguard-monitor.service` con `Restart=always`.

### Verificación

```bash
systemd-analyze verify /etc/systemd/system/sleepguard-guard.service
sudo SLEEPGUARD_DRY_RUN=1 systemctl start sleepguard-guard
journalctl -u sleepguard-guard -e
```

CLI del agente:

```bash
SLEEPGUARD_DRY_RUN=1 sleepguard-guard --now 06:30   # mensaje + countdown + dry-run
SLEEPGUARD_DRY_RUN=1 sleepguard-guard --now 09:00   # exit 0 (ya pasado activation)
```

## Windows

PowerShell **como Administrador**:

```powershell
# Solo guard (recomendado si el monitor ya lo registró la app)
.\installer\windows\register-guard.ps1 -GuardOnly -AppPath "C:\Path\to\sleepguard-app.exe"

# Guard + monitor
.\installer\windows\register-guard.ps1 -AppPath "C:\Path\to\sleepguard-app.exe"

# Solo monitor (sin elevación alta; la app también lo hace sola)
.\installer\windows\register-guard.ps1 -MonitorOnly -AppPath "C:\Path\to\sleepguard-app.exe"
```

| Tarea | Comando | Notas |
|---|---|---|
| `SleepGuard-Monitor` | `sleepguard-app.exe` | Restart on failure; RunLevel Limited |
| `SleepGuard-Guard` | `sleepguard-app.exe --guard` | RunLevel Highest |

## Desde la UI

Al **activar el servicio**, SleepGuard muestra un diálogo para permitir instalar el agente de arranque (necesario para bloquear el PC si se enciende antes de la hora de activación). Si el usuario acepta:

- Linux: `pkexec` + `install-guard.sh --guard-only`
- Windows: PowerShell elevado + `register-guard.ps1 -GuardOnly`

Si elige «Ahora no», el horario de apagado sigue activo y la hora de activación queda bloqueada hasta que active el **Agente de arranque** en Configuración avanzada.

## Limitaciones conocidas (v1.1)

### RNF-02 — Servicio Windows

El monitor **no** es un Windows Service real (no requiere credenciales de admin para terminar el proceso). Mitigación parcial: Task Scheduler re-lanza on-failure.

### Dry-run global

```bash
SLEEPGUARD_DRY_RUN=1 npm run tauri dev
# o
SLEEPGUARD_DRY_RUN=1 ./sleepguard-app --guard
```

`execute_shutdown*` solo escribe `[DRY-RUN] shutdown (...)` en el log.
