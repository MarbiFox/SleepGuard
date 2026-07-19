import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

interface ShutdownLockscreenPayload {
  activation_time: string;
  countdown_secs: number;
}

function App() {
  const [currentScreen, setCurrentScreen] = useState<Screen | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [bootGuardActivation, setBootGuardActivation] = useState<string | null>(null);
  const [scheduledLockscreen, setScheduledLockscreen] = useState<ShutdownLockscreenPayload | null>(
    null
  );
  const screenBeforeLockscreen = useRef<Screen>("main");
  const [bootGuardEpoch, setBootGuardEpoch] = useState(0);

  useEffect(() => {
    async function bootstrap() {
      try {
        const launch = await invoke<LaunchModeResponse>("get_launch_mode");
        if (launch.mode === "guard" && launch.activation) {
          setBootGuardActivation(launch.activation);
          setCurrentScreen("lockscreen");
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

  useEffect(() => {
    let unlistenShow: (() => void) | undefined;
    let unlistenDismiss: (() => void) | undefined;

    listen<ShutdownLockscreenPayload>("show-shutdown-lockscreen", (event) => {
      setScheduledLockscreen(event.payload);
      setCurrentScreen((prev) => {
        if (prev && prev !== "lockscreen") {
          screenBeforeLockscreen.current = prev;
        }
        return "lockscreen";
      });
    }).then((fn) => {
      unlistenShow = fn;
    });

    listen("dismiss-shutdown-lockscreen", () => {
      setScheduledLockscreen(null);
      setCurrentScreen((prev) =>
        prev === "lockscreen" ? screenBeforeLockscreen.current : prev
      );
    }).then((fn) => {
      unlistenDismiss = fn;
    });

    return () => {
      unlistenShow?.();
      unlistenDismiss?.();
    };
  }, []);

  const saveConfig = async (newConfig: AppConfig) => {
    try {
      await invoke("save_config", { config: newConfig });
      setConfig(newConfig);
      await invoke("ensure_monitor_autostart", { enabled: newConfig.enabled });
    } catch (err) {
      console.error("Failed to save config:", err);
    }
  };

  const finishOnboarding = async (newConfig: AppConfig) => {
    await saveConfig(newConfig);
    setCurrentScreen("main");
  };

  const installBootGuard = async (): Promise<boolean> => {
    try {
      await invoke("ensure_boot_guard");
      return true;
    } catch (err) {
      console.error("Failed to install boot guard:", err);
      return false;
    }
  };

  if (!config || !currentScreen) {
    return <div style={{ color: "white" }}>Cargando configuración...</div>;
  }

  if (bootGuardActivation) {
    return (
      <Lockscreen
        mode="real"
        activationTime={bootGuardActivation}
        initialCountdown={30}
      />
    );
  }

  if (scheduledLockscreen) {
    return (
      <Lockscreen
        mode="real"
        activationTime={scheduledLockscreen.activation_time}
        initialCountdown={scheduledLockscreen.countdown_secs}
      />
    );
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
          onPreviewLockscreen={() => {
            screenBeforeLockscreen.current = "main";
            setCurrentScreen("lockscreen");
          }}
          onInstallBootGuard={installBootGuard}
          bootGuardEpoch={bootGuardEpoch}
        />
      )}
      {currentScreen === "advanced" && (
        <Advanced
          config={config}
          onSave={saveConfig}
          onBack={() => setCurrentScreen("main")}
          onInstallBootGuard={installBootGuard}
          onBootGuardPrefsChanged={() => setBootGuardEpoch((n) => n + 1)}
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
