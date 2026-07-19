# SleepGuard Windows installer — Task Scheduler (RF-07, RNF-05).
# Run elevated (Run as Administrator) when registering the Guard task.
#Requires -RunAsAdministrator

param(
    [string]$AppPath = "",
    [string]$ConfigPath = "",
    [switch]$GuardOnly,
    [switch]$MonitorOnly
)

$ErrorActionPreference = "Stop"

if (-not $AppPath) {
    $candidates = @(
        (Join-Path $PSScriptRoot "..\..\src-tauri\target\release\sleepguard-app.exe"),
        (Join-Path $PSScriptRoot "sleepguard-app.exe"),
        (Get-Command sleepguard-app.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            $AppPath = (Resolve-Path $candidate).Path
            break
        }
    }
    if (-not $AppPath) {
        Write-Error "No se encontró sleepguard-app.exe. Pasa -AppPath o compila en release."
    }
}

if (-not $ConfigPath) {
    $ConfigPath = Join-Path $env:APPDATA "sleepguard\config.json"
}

$configDir = Split-Path $ConfigPath -Parent
if (-not (Test-Path $configDir)) {
    New-Item -ItemType Directory -Path $configDir -Force | Out-Null
}

function Register-SleepGuardTask {
    param(
        [string]$Name,
        [string]$Arguments,
        [string]$Description,
        [ValidateSet("Limited", "Highest")]
        [string]$RunLevel = "Highest"
    )

    Unregister-ScheduledTask -TaskName $Name -Confirm:$false -ErrorAction SilentlyContinue

    $action = New-ScheduledTaskAction -Execute $AppPath -Argument $Arguments
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    $settings = New-ScheduledTaskSettingsSet `
        -AllowStartIfOnBatteries `
        -DontStopIfGoingOnBatteries `
        -StartWhenAvailable `
        -RestartCount 3 `
        -RestartInterval (New-TimeSpan -Minutes 1) `
        -ExecutionTimeLimit (New-TimeSpan -Days 0)
    $settings.Priority = 4

    $principal = New-ScheduledTaskPrincipal `
        -UserId $env:USERNAME `
        -LogonType Interactive `
        -RunLevel $RunLevel

    Register-ScheduledTask `
        -TaskName $Name `
        -Action $action `
        -Trigger $trigger `
        -Settings $settings `
        -Principal $principal `
        -Description $Description | Out-Null
}

$doMonitor = -not $GuardOnly
$doGuard = -not $MonitorOnly
if ($GuardOnly -and $MonitorOnly) {
    Write-Error "No combines -GuardOnly y -MonitorOnly."
}

if ($doMonitor) {
    Register-SleepGuardTask `
        -Name "SleepGuard-Monitor" `
        -Arguments "--background" `
        -Description "SleepGuard monitor at logon (Restart on failure)" `
        -RunLevel Limited
}

if ($doGuard) {
    Register-SleepGuardTask `
        -Name "SleepGuard-Guard" `
        -Arguments "--guard" `
        -Description "SleepGuard activation guard at logon" `
        -RunLevel Highest
}

Write-Host ""
Write-Host "Tareas registradas:"
if ($doMonitor) {
    Write-Host "  SleepGuard-Monitor  -> $AppPath --background"
}
if ($doGuard) {
    Write-Host "  SleepGuard-Guard    -> $AppPath --guard"
}
Write-Host "  Config esperado:    $ConfigPath"
Write-Host ""
Write-Host "Limitacion v1.0 (RNF-02): el monitor NO es un Windows Service real;"
Write-Host "Task Scheduler re-lanza on-failure (mitigacion parcial)."
Write-Host ""
Write-Host "Prueba en seco: `$env:SLEEPGUARD_DRY_RUN=1; & '$AppPath' --guard"
