/** Format raw digits into HH:MM as the user types. */
export function formatTimeInput(value: string): string {
  let val = value.replace(/\D/g, "");
  if (val.length >= 3) {
    val = val.slice(0, 2) + ":" + val.slice(2, 4);
  }
  return val;
}

/** Clamp a completed HH:MM value to valid hours/minutes. */
export function clampTimeOnBlur(value: string): string | null {
  const trimmed = value.trim();
  if (trimmed.length !== 5 || !trimmed.includes(":")) {
    return null;
  }

  const [hours, minutes] = trimmed.split(":");
  let h = parseInt(hours, 10);
  let m = parseInt(minutes, 10);

  if (Number.isNaN(h) || Number.isNaN(m)) {
    return null;
  }

  if (h > 23) h = 23;
  if (m > 59) m = 59;
  if (h < 0) h = 0;
  if (m < 0) m = 0;

  return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}`;
}

export function handleTimeBlur(
  value: string,
  setter: (val: string) => void,
): void {
  const clamped = clampTimeOnBlur(value);
  if (clamped !== null) {
    setter(clamped);
  }
}
