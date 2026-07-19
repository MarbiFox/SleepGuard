import { useState, useEffect } from "react";
import { AppConfig } from "../App";
import { formatTimeInput, handleTimeBlur } from "../utils/time";
import {
  canEditActivation,
  markBootGuardDeclined,
  markBootGuardInstalled,
  shouldPromptBootGuardOnEnable,
} from "../utils/bootGuard";

interface MainProps {
  config: AppConfig;
  onSave: (cfg: AppConfig) => void;
  onGoToAdvanced: () => void;
  onPreviewLockscreen: () => void;
  onInstallBootGuard: () => Promise<boolean>;
  /** Bumped when Advanced changes boot-guard prefs so Main re-reads localStorage. */
  bootGuardEpoch: number;
}

export default function Main({
  config,
  onSave,
  onGoToAdvanced,
  onPreviewLockscreen,
  onInstallBootGuard,
  bootGuardEpoch,
}: MainProps) {
  const [enabled, setEnabled] = useState(config.enabled);
  const [timeOff, setTimeOff] = useState(config.schedule.shutdown_default);
  const [timeOn, setTimeOn] = useState(config.schedule.activation_default);
  const [activationEditable, setActivationEditable] = useState(canEditActivation);
  const [showBootGuardPrompt, setShowBootGuardPrompt] = useState(false);
  const [installingBootGuard, setInstallingBootGuard] = useState(false);
  const [bootGuardMessage, setBootGuardMessage] = useState<string | null>(null);

  useEffect(() => {
    setActivationEditable(canEditActivation());
  }, [bootGuardEpoch]);

  useEffect(() => {
    if (
      enabled !== config.enabled ||
      timeOff !== config.schedule.shutdown_default ||
      timeOn !== config.schedule.activation_default
    ) {
      const enabling = enabled && !config.enabled;
      onSave({
        ...config,
        enabled,
        schedule: {
          ...config.schedule,
          shutdown_default: timeOff,
          activation_default: timeOn,
        },
      });

      if (enabling && shouldPromptBootGuardOnEnable()) {
        setBootGuardMessage(null);
        setShowBootGuardPrompt(true);
      }
    }
  }, [enabled, timeOff, timeOn]);

  const handleAllowBootGuard = async () => {
    setInstallingBootGuard(true);
    setBootGuardMessage("Solicitando permisos de administrador…");
    const ok = await onInstallBootGuard();
    setInstallingBootGuard(false);
    if (ok) {
      markBootGuardInstalled();
      setActivationEditable(true);
      setBootGuardMessage(null);
      setShowBootGuardPrompt(false);
    } else {
      setBootGuardMessage(
        "No se pudo instalar el agente. Puedes activarlo más tarde en Configuración avanzada."
      );
    }
  };

  const handleSkipBootGuard = () => {
    markBootGuardDeclined();
    setActivationEditable(false);
    setShowBootGuardPrompt(false);
    setBootGuardMessage(null);
  };

  return (
    <>
      <div className="preview-btn-container">
        <button className="btn-secondary" onClick={onPreviewLockscreen}>
          Vista previa del bloqueo
        </button>
      </div>
      <main className="app-card">
        <header className="app-header">
          <h1 className="app-title">SleepGuard</h1>
        </header>

        <div className="toggle-section">
          <label
            className="switch-container"
            aria-label="Habilitar o deshabilitar SleepGuard"
          >
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
            />
            <div className="switch-track"></div>
          </label>
          <div className={`toggle-status ${!enabled ? "disabled" : ""}`}>
            {enabled ? "Servicio activado" : "Servicio pausado"}
          </div>
        </div>

        <div className={`time-inputs ${!enabled ? "disabled" : ""}`}>
          <div className="input-group-main">
            <label htmlFor="time-off">Hora de apagado (default)</label>
            <input
              type="text"
              id="time-off"
              className="time-display"
              value={timeOff}
              maxLength={5}
              inputMode="numeric"
              onChange={(e) => setTimeOff(formatTimeInput(e.target.value))}
              onBlur={(e) => handleTimeBlur(e.target.value, setTimeOff)}
              tabIndex={enabled ? 0 : -1}
            />
          </div>

          <div
            className={`input-group-main ${!activationEditable ? "field-locked" : ""}`}
          >
            <label htmlFor="time-on">Hora de activación (default)</label>
            <input
              type="text"
              id="time-on"
              className="time-display"
              value={timeOn}
              maxLength={5}
              inputMode="numeric"
              disabled={!enabled || !activationEditable}
              onChange={(e) => setTimeOn(formatTimeInput(e.target.value))}
              onBlur={(e) => handleTimeBlur(e.target.value, setTimeOn)}
              tabIndex={enabled && activationEditable ? 0 : -1}
            />
            {!activationEditable && (
              <p className="field-hint">
                Requiere el agente de arranque (Configuración avanzada).
              </p>
            )}
          </div>
        </div>

        <footer className="app-footer">
          <button className="btn-secondary" onClick={onGoToAdvanced}>
            Configuración avanzada
          </button>
        </footer>
      </main>

      {showBootGuardPrompt && (
        <div className="modal-backdrop" role="presentation">
          <div
            className="modal-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="boot-guard-title"
          >
            <h2 id="boot-guard-title" className="modal-title">
              Instalar agente de arranque
            </h2>
            <p className="modal-body">
              SleepGuard necesita un agente de arranque para bloquear el PC si se
              enciende antes de la hora de activación. Sin él, el horario de
              apagado sigue funcionando, pero no habrá protección al iniciar el
              sistema.
            </p>
            <p className="modal-body modal-body-muted">
              Se pedirán permisos de administrador una sola vez. Si eliges «Ahora
              no», podrás activarlo después en Configuración avanzada.
            </p>
            {bootGuardMessage && (
              <p className="modal-status">{bootGuardMessage}</p>
            )}
            <div className="modal-actions">
              <button
                type="button"
                className="btn-secondary"
                onClick={handleSkipBootGuard}
                disabled={installingBootGuard}
              >
                Ahora no
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={() => void handleAllowBootGuard()}
                disabled={installingBootGuard}
              >
                {installingBootGuard ? "Instalando…" : "Permitir"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
