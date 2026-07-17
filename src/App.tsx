import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import Main from "./screens/Main";
import Advanced from "./screens/Advanced";
import Lockscreen from "./screens/Lockscreen";
import Onboarding from "./screens/Onboarding";

type Screen = "onboarding" | "main" | "advanced" | "lockscreen";

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

interface LaunchModeResponse {
  mode: "normal" | "guard";
  activation?: string;
}

function App() {
  const [currentScreen, setCurrentScreen] = useState<Screen | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [guardActivation, setGuardActivation] = useState<string | null>(null);

  useEffect(() => {
    async function bootstrap() {
      try {
        const launch = await invoke<LaunchModeResponse>("get_launch_mode");
        if (launch.mode === "guard" && launch.activation) {
          setGuardActivation(launch.activation);
          setCurrentScreen("lockscreen");
          // Still load config for consistency, but UI is locked to guard
          const cfg = await invoke<AppConfig>("load_config");
          setConfig(cfg);
          return;
        }

        const first = await invoke<boolean>("is_first_launch");
        const cfg = await invoke<AppConfig>("load_config");
        setConfig(cfg);
        setCurrentScreen(first ? "onboarding" : "main");
      } catch (err) {
        console.error("Failed to bootstrap:", err);
      }
    }

    bootstrap();
  }, []);

  const saveConfig = async (newConfig: AppConfig) => {
    try {
      await invoke("save_config", { config: newConfig });
      setConfig(newConfig);
    } catch (err) {
      console.error("Failed to save config:", err);
    }
  };

  const finishOnboarding = async (newConfig: AppConfig) => {
    await saveConfig(newConfig);
    setCurrentScreen("main");
  };

  if (!config || !currentScreen) {
    return <div style={{ color: "white" }}>Cargando configuración...</div>;
  }

  if (guardActivation) {
    return <Lockscreen mode="real" activationTime={guardActivation} />;
  }

  return (
    <>
      {currentScreen === "onboarding" && (
        <Onboarding
          detectedOs={config.os}
          config={config}
          onConfirm={finishOnboarding}
        />
      )}
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
          mode="preview"
          activationTime={config.schedule.activation_default || "07:00"}
          onUnlockTest={() => setCurrentScreen("main")}
        />
      )}
    </>
  );
}

export default App;
