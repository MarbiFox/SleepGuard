import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LockscreenProps {
  mode: "preview" | "real";
  activationTime: string;
  initialCountdown?: number;
  onUnlockTest?: () => void;
}

export default function Lockscreen({
  mode,
  activationTime,
  initialCountdown = 30,
  onUnlockTest,
}: LockscreenProps) {
  const [timeLeft, setTimeLeft] = useState(initialCountdown);
  const shutdownFired = useRef(false);

  useEffect(() => {
    const preventContext = (e: MouseEvent) => e.preventDefault();
    const preventKeys = (e: KeyboardEvent) => {
      if (mode === "real") {
        e.preventDefault();
        return;
      }
      if (e.key !== "Escape") e.preventDefault();
    };

    window.addEventListener("contextmenu", preventContext);
    window.addEventListener("keydown", preventKeys);

    return () => {
      window.removeEventListener("contextmenu", preventContext);
      window.removeEventListener("keydown", preventKeys);
    };
  }, [mode]);

  useEffect(() => {
    if (timeLeft <= 0) {
      if (mode === "real" && !shutdownFired.current) {
        shutdownFired.current = true;
        invoke("execute_shutdown_now").catch((err) =>
          console.error("Failed to execute shutdown:", err)
        );
      }
      return;
    }

    const timer = setInterval(() => {
      setTimeLeft((prev) => prev - 1);
    }, 1000);

    return () => clearInterval(timer);
  }, [timeLeft, mode]);

  return (
    <main className={`lock-container window ${mode}`}>
      {mode === "preview" && onUnlockTest && (
        <button
          onClick={onUnlockTest}
          style={{
            position: "absolute",
            top: 20,
            right: 20,
            opacity: 0.55,
            background: "transparent",
            color: "#fff",
            border: "none",
            padding: "8px 12px",
            cursor: "pointer",
            fontSize: "0.875rem",
          }}
        >
          Exit Test
        </button>
      )}

      <div className="lock-icon-wrapper">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
        </svg>
      </div>

      <h1 className="headline">
        Este equipo estará disponible a las{" "}
        <span className="time-target">{activationTime}</span>
      </h1>

      <div className="countdown">
        Apagando en <span>{timeLeft.toString().padStart(2, "0")}</span>s...
      </div>
    </main>
  );
}
