import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import Main from "./screens/Main";
import Advanced from "./screens/Advanced";
import Lockscreen from "./screens/Lockscreen";

type Screen = "main" | "advanced" | "lockscreen";

export interface OverrideConfig {
  shutdown: string;
  activation: string;
}

export interface ScheduleConfig {
  shutdown_default: string;
  activation_default: string;
  overrides: Record<string, OverrideConfig>;
}

export interface AppConfig {
  os: string;
  enabled: boolean;
  schedule: ScheduleConfig;
}

function App() {
  const [currentScreen, setCurrentScreen] = useState<Screen>("main");
  const [config, setConfig] = useState<AppConfig | null>(null);

  useEffect(() => {
    // Cargar configuración al iniciar
    invoke<AppConfig>("load_config")
      .then((cfg) => setConfig(cfg))
      .catch((err) => console.error("Failed to load config:", err));
  }, []);

  const saveConfig = async (newConfig: AppConfig) => {
    try {
      await invoke("save_config", { config: newConfig });
      setConfig(newConfig);
    } catch (err) {
      console.error("Failed to save config:", err);
    }
  };

  if (!config) {
    return <div style={{color: "white"}}>Cargando configuración...</div>;
  }

  return (
    <>
      {currentScreen === "main" && (
        <Main 
          config={config}
          onSave={saveConfig}
          onGoToAdvanced={() => setCurrentScreen("advanced")} 
          onPreviewLockscreen={() => setCurrentScreen("lockscreen")} 
        />
      )}
      {currentScreen === "advanced" && (
        <Advanced 
          config={config}
          onSave={saveConfig}
          onBack={() => setCurrentScreen("main")} 
        />
      )}
      {currentScreen === "lockscreen" && (
        <Lockscreen 
          onUnlockTest={() => setCurrentScreen("main")} 
        />
      )}
    </>
  );
}

export default App;
