import { useState, useEffect } from "react";
import { AppConfig } from "../App";

interface MainProps {
  config: AppConfig;
  onSave: (cfg: AppConfig) => void;
  onGoToAdvanced: () => void;
  onPreviewLockscreen: () => void;
}

export default function Main({ config, onSave, onGoToAdvanced, onPreviewLockscreen }: MainProps) {
  const [enabled, setEnabled] = useState(config.enabled);
  const [timeOff, setTimeOff] = useState(config.schedule.shutdown_default);
  const [timeOn, setTimeOn] = useState(config.schedule.activation_default);

  // Guardar configuración cuando se cambie algo desde aquí
  useEffect(() => {
    if (
      enabled !== config.enabled ||
      timeOff !== config.schedule.shutdown_default ||
      timeOn !== config.schedule.activation_default
    ) {
      onSave({
        ...config,
        enabled,
        schedule: {
          ...config.schedule,
          shutdown_default: timeOff,
          activation_default: timeOn,
        },
      });
    }
  }, [enabled, timeOff, timeOn]);

  const formatTimeInput = (value: string) => {
    let val = value.replace(/\D/g, "");
    if (val.length >= 3) {
      val = val.slice(0, 2) + ":" + val.slice(2, 4);
    }
    return val;
  };

  const handleBlur = (value: string, setter: (val: string) => void) => {
    if (value.length === 5) {
      let [hours, minutes] = value.split(":");
      let h = parseInt(hours, 10);
      let m = parseInt(minutes, 10);

      if (h > 23) h = 23;
      if (m > 59) m = 59;

      setter(`${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}`);
    }
  };

  return (
    <>
      <div className="preview-btn-container">
        <button className="btn-secondary" onClick={onPreviewLockscreen}>Preview Lockscreen</button>
      </div>
      <main className="app-card">
        <header className="app-header">
          <h1 className="app-title">SleepGuard</h1>
        </header>

        <div className="toggle-section">
          <label className="switch-container" aria-label="Habilitar o deshabilitar SleepGuard">
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
              onBlur={(e) => handleBlur(e.target.value, setTimeOff)}
              tabIndex={enabled ? 0 : -1}
            />
          </div>

          <div className="input-group-main">
            <label htmlFor="time-on">Hora de activación (default)</label>
            <input
              type="text"
              id="time-on"
              className="time-display"
              value={timeOn}
              maxLength={5}
              inputMode="numeric"
              onChange={(e) => setTimeOn(formatTimeInput(e.target.value))}
              onBlur={(e) => handleBlur(e.target.value, setTimeOn)}
              tabIndex={enabled ? 0 : -1}
            />
          </div>
        </div>

        <footer className="app-footer">
          <button className="btn-secondary" onClick={onGoToAdvanced}>
            Configuración avanzada
          </button>
        </footer>
      </main>
    </>
  );
}
