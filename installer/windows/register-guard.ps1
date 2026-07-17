# SleepGuard Windows installer — Task Scheduler autostart (RF-07, RNF-05).
# Run elevated (Run as Administrator).
#Requires -RunAsAdministrator

param(
    [string]$AppPath = "",
    [string]$ConfigPath = ""
)

$ErrorActionPreference = "Stop"

if (-not $AppPath) {
    $candidate = Join-Path $PSScriptRoot "..\..\src-tauri\target\release\sleepguard-app.exe"
    if (Test-Path $candidate) {
        $AppPath = (Resolve-Path $candidate).Path
    } else {
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
        [string]$Description
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
    # Highest available priority for interactive tasks
    $settings.Priority = 4

    $principal = New-ScheduledTaskPrincipal `
        -UserId $env:USERNAME `
        -LogonType Interactive `
        -RunLevel Highest

    Register-ScheduledTask `
        -TaskName $Name `
        -Action $action `
        -Trigger $trigger `
        -Settings $settings `
        -Principal $principal `
        -Description $Description | Out-Null
}

# Task 1: monitor (GUI + background monitor loop)
Register-SleepGuardTask `
    -Name "SleepGuard-Monitor" `
    -Arguments "" `
    -Description "SleepGuard monitor at logon (Restart on failure)"

# Task 2: --guard lockscreen at logon (activation check)
Register-SleepGuardTask `
    -Name "SleepGuard-Guard" `
    -Arguments "--guard" `
    -Description "SleepGuard activation guard at logon"

Write-Host ""
Write-Host "Tareas registradas:"
Write-Host "  SleepGuard-Monitor  -> $AppPath"
Write-Host "  SleepGuard-Guard    -> $AppPath --guard"
Write-Host "  Config esperado:    $ConfigPath"
Write-Host ""
Write-Host "Limitacion v1.0 (RNF-02): el monitor NO es un Windows Service real;"
Write-Host "Task Scheduler re-lanza on-failure (mitigacion parcial)."
Write-Host ""
Write-Host "Prueba en seco: `$env:SLEEPGUARD_DRY_RUN=1; & '$AppPath' --guard"
