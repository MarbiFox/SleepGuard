import { useState } from "react";
import { AppConfig } from "../App";

interface OnboardingProps {
  detectedOs: string;
  config: AppConfig;
  onConfirm: (cfg: AppConfig) => void;
}

export default function Onboarding({ detectedOs, config, onConfirm }: OnboardingProps) {
  const [manual, setManual] = useState(false);
  const [os, setOs] = useState(detectedOs === "windows" ? "windows" : "linux");

  const continueWith = (chosen: string) => {
    onConfirm({
      ...config,
      os: chosen,
    });
  };

  return (
    <main className="app-card onboarding-card">
      <header className="app-header">
        <h1 className="app-title">SleepGuard</h1>
        <p className="onboarding-subtitle">Configuración inicial</p>
      </header>

      <div className="onboarding-body">
        {!manual ? (
          <>
            <p className="onboarding-text">
              Se detectó <strong>{detectedOs === "windows" ? "Windows" : "Linux"}</strong>.
              ¿Es correcto?
            </p>
            <div className="onboarding-actions">
              <button className="primary-btn" onClick={() => continueWith(os)}>
                Sí, continuar
              </button>
              <button className="btn-secondary" onClick={() => setManual(true)}>
                Cambiar manualmente
              </button>
            </div>
          </>
        ) : (
          <>
            <p className="onboarding-text">Selecciona el sistema operativo:</p>
            <label className="onboarding-select-label" htmlFor="os-select">
              Sistema operativo
            </label>
            <select
              id="os-select"
              className="onboarding-select"
              value={os}
              onChange={(e) => setOs(e.target.value)}
            >
              <option value="linux">Linux</option>
              <option value="windows">Windows</option>
            </select>
            <div className="onboarding-actions">
              <button className="primary-btn" onClick={() => continueWith(os)}>
                Confirmar
              </button>
              <button className="btn-secondary" onClick={() => setManual(false)}>
                Volver
              </button>
            </div>
          </>
        )}
      </div>
    </main>
  );
}
