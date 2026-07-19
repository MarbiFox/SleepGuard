const INSTALLED_KEY = "sleepguard_boot_guard_installed";
const DECLINED_KEY = "sleepguard_boot_guard_declined";

export function isBootGuardInstalled(): boolean {
  return localStorage.getItem(INSTALLED_KEY) === "1";
}

export function isBootGuardDeclined(): boolean {
  return localStorage.getItem(DECLINED_KEY) === "1";
}

/** Activation schedule is editable only when the boot agent is installed (or not declined yet). */
export function canEditActivation(): boolean {
  if (isBootGuardInstalled()) return true;
  return !isBootGuardDeclined();
}

export function markBootGuardInstalled(): void {
  localStorage.setItem(INSTALLED_KEY, "1");
  localStorage.removeItem(DECLINED_KEY);
}

export function markBootGuardDeclined(): void {
  localStorage.setItem(DECLINED_KEY, "1");
  localStorage.removeItem(INSTALLED_KEY);
}

export function shouldPromptBootGuardOnEnable(): boolean {
  return !isBootGuardInstalled() && !isBootGuardDeclined();
}
