import { useState, useEffect } from "react";

interface LockscreenProps {
  onUnlockTest: () => void; // Solo para poder probar salir de la pantalla de bloqueo
}

export default function Lockscreen({ onUnlockTest }: LockscreenProps) {
  const [timeLeft, setTimeLeft] = useState(30);

  useEffect(() => {
    // Bloquear el click derecho y teclas comunes para testear visualmente (aunque en app nativa lo hace el SO)
    const preventContext = (e: MouseEvent) => e.preventDefault();
    const preventKeys = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') e.preventDefault(); // Permitir Escape solo para la prueba
    };

    window.addEventListener("contextmenu", preventContext);
    window.addEventListener("keydown", preventKeys);

    return () => {
      window.removeEventListener("contextmenu", preventContext);
      window.removeEventListener("keydown", preventKeys);
    };
  }, []);

  useEffect(() => {
    if (timeLeft <= 0) return;
    
    const timer = setInterval(() => {
      setTimeLeft((prev) => prev - 1);
    }, 1000);

    return () => clearInterval(timer);
  }, [timeLeft]);

  return (
    <main className="lock-container window">
      {/* Botón oculto/emergencia para salir de la demo */}
      <button 
        onClick={onUnlockTest} 
        style={{ position: 'absolute', top: 20, right: 20, opacity: 0.1, background: 'transparent', color: '#fff', border: '1px solid #fff' }}
      >
        Exit Test
      </button>

      <div className="lock-icon-wrapper">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
        </svg>
      </div>

      <h1 className="headline">
        Este equipo estará disponible a las <span className="time-target">07:00</span>
      </h1>

      <div className="countdown">
        Apagando en <span>{timeLeft.toString().padStart(2, '0')}</span>s...
      </div>
    </main>
  );
}
