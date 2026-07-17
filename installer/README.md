# SleepGuard — Instalación y autostart (v1.0)

Scripts explícitos y auditables para RF-07, RNF-01 y RNF-05.

## Requisitos previos

Compila en release:

```bash
cd src-tauri
cargo build --release -p sleepguard-core
cargo build --release -p sleepguard-guard
cargo build --release -p sleepguard-app
```

## Linux

```bash
sudo ./installer/linux/install-guard.sh
```

Instala:

1. `/usr/local/bin/sleepguard-guard` — unidad systemd de sistema (`sleepguard-guard.service`) **antes** del display manager.
2. User unit `sleepguard-monitor.service` con `Restart=always` (RNF-05).

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

Ejecutar PowerShell **como Administrador**:

```powershell
.\installer\windows\register-guard.ps1
# o con ruta explícita:
.\installer\windows\register-guard.ps1 -AppPath "C:\Path\to\sleepguard-app.exe"
```

Registra dos tareas al logon:

| Tarea | Comando | Notas |
|---|---|---|
| `SleepGuard-Monitor` | `sleepguard-app.exe` | Restart on failure |
| `SleepGuard-Guard` | `sleepguard-app.exe --guard` | RunLevel Highest |

## Limitaciones conocidas (v1.0)

### RNF-02 — Servicio Windows

El monitor **no** es un Windows Service real (no requiere credenciales de admin para terminar el proceso). Mitigación parcial: Task Scheduler re-lanza on-failure.

Un service host dedicado queda fuera del alcance de v1.0.

### Mejora futura

Command Tauri “instalar agente” con `pkexec` / PowerShell elevado — no incluido en v1.0; los scripts de este directorio son la vía soportada.

## Dry-run global

```bash
SLEEPGUARD_DRY_RUN=1 npm run tauri dev
# o
SLEEPGUARD_DRY_RUN=1 ./sleepguard-app --guard
```

`execute_shutdown*` solo escribe `[DRY-RUN] shutdown (...)` en el log.
