import { useState } from "react";
import { AppConfig, OverrideConfig } from "../App";
import { formatTimeInput, handleTimeBlur } from "../utils/time";
import {
  canEditActivation,
  isBootGuardInstalled,
  markBootGuardDeclined,
  markBootGuardInstalled,
} from "../utils/bootGuard";

interface AdvancedProps {
  config: AppConfig;
  onSave: (cfg: AppConfig) => void;
  onBack: () => void;
  onInstallBootGuard: () => Promise<string | null>;
  onBootGuardPrefsChanged: () => void;
}

const DAYS = [
  { id: "mon", name: "Lunes" },
  { id: "tue", name: "Martes" },
  { id: "wed", name: "Miércoles" },
  { id: "thu", name: "Jueves" },
  { id: "fri", name: "Viernes" },
  { id: "sat", name: "Sábado" },
  { id: "sun", name: "Domingo" },
];

export default function Advanced({
  config,
  onSave,
  onBack,
  onInstallBootGuard,
  onBootGuardPrefsChanged,
}: AdvancedProps) {
  const [schedule, setSchedule] = useState<Record<string, OverrideConfig>>(
    config.schedule.overrides
  );
  const [bootGuardOn, setBootGuardOn] = useState(isBootGuardInstalled);
  const [activationEditable, setActivationEditable] = useState(canEditActivation);
  const [installing, setInstalling] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const handleChange = (day: string, field: "shutdown" | "activation", value: string) => {
    if (field === "activation" && !activationEditable) return;
    setSchedule((prev) => ({
      ...prev,
      [day]: {
        ...(prev[day] || { shutdown: "", activation: "" }),
        [field]: formatTimeInput(value),
      },
    }));
  };

  const handleBlur = (day: string, field: "shutdown" | "activation", value: string) => {
    if (field === "activation" && !activationEditable) return;
    handleTimeBlur(value, (clamped) => {
      setSchedule((prev) => ({
        ...prev,
        [day]: {
          ...(prev[day] || { shutdown: "", activation: "" }),
          [field]: clamped,
        },
      }));
    });
  };

  const clearDay = (day: string) => {
    setSchedule((prev) => ({
      ...prev,
      [day]: {
        shutdown: "",
        activation: activationEditable ? "" : prev[day]?.activation || "",
      },
    }));
  };

  const handleBootGuardToggle = async (checked: boolean) => {
    if (checked) {
      setInstalling(true);
      setStatus("Solicitando permisos de administrador…");
      const error = await onInstallBootGuard();
      setInstalling(false);
      if (!error) {
        markBootGuardInstalled();
        setBootGuardOn(true);
        setActivationEditable(true);
        setStatus("Agente de arranque instalado.");
        onBootGuardPrefsChanged();
      } else {
        setBootGuardOn(false);
        setStatus(error);
      }
      return;
    }

    markBootGuardDeclined();
    setBootGuardOn(false);
    setActivationEditable(false);
    setStatus(null);
    onBootGuardPrefsChanged();
  };

  const handleSave = () => {
    onSave({
      ...config,
      schedule: {
        ...config.schedule,
        overrides: schedule,
      },
    });
    onBack();
  };

  return (
    <div className="card app-card advanced">
      <header className="header-adv">
        <button className="back-btn" aria-label="Volver" onClick={onBack}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M19 12H5"></path>
            <polyline points="12 19 5 12 12 5"></polyline>
          </svg>
        </button>
        <h1 className="title-adv">Horarios específicos por día</h1>
      </header>

      <div className="content">
        <div className="adv-boot-guard">
          <div className="adv-boot-guard-text">
            <span className="adv-boot-guard-title">Agente de arranque</span>
            <span className="adv-boot-guard-desc">
              Bloquea el PC si se enciende antes de la hora de activación. Requiere
              permisos de administrador.
            </span>
            {status && <span className="adv-boot-guard-status">{status}</span>}
          </div>
          <label
            className="switch-container switch-container-sm"
            aria-label="Activar o desactivar agente de arranque"
          >
            <input
              type="checkbox"
              checked={bootGuardOn}
              disabled={installing}
              onChange={(e) => void handleBootGuardToggle(e.target.checked)}
            />
            <div className="switch-track"></div>
          </label>
        </div>

        <div className="day-list">
          {DAYS.map((day) => {
            const dayConfig = schedule[day.id] || { shutdown: "", activation: "" };
            return (
              <div className="day-row" key={day.id}>
                <span className="day-name">{day.name}</span>
                <div className="day-inputs">
                  <div className="input-group-adv">
                    <label>Apagado</label>
                    <input
                      type="text"
                      className="time-input-adv"
                      placeholder={config.schedule.shutdown_default}
                      value={dayConfig.shutdown}
                      maxLength={5}
                      onChange={(e) => handleChange(day.id, "shutdown", e.target.value)}
                      onBlur={(e) => handleBlur(day.id, "shutdown", e.target.value)}
                    />
                  </div>
                  <div
                    className={`input-group-adv ${!activationEditable ? "field-locked" : ""}`}
                  >
                    <label>Activación</label>
                    <input
                      type="text"
                      className="time-input-adv"
                      placeholder={config.schedule.activation_default}
                      value={dayConfig.activation}
                      maxLength={5}
                      disabled={!activationEditable}
                      onChange={(e) => handleChange(day.id, "activation", e.target.value)}
                      onBlur={(e) => handleBlur(day.id, "activation", e.target.value)}
                    />
                  </div>
                  <button
                    className="clear-btn"
                    aria-label={`Limpiar ${day.name}`}
                    title="Limpiar"
                    onClick={() => clearDay(day.id)}
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <polyline points="3 6 5 6 21 6"></polyline>
                      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                    </svg>
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <footer className="footer-adv">
        <button className="primary-btn" onClick={handleSave}>
          Guardar cambios
        </button>
      </footer>
    </div>
  );
}
