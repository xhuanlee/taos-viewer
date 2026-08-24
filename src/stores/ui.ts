import { defineStore } from "pinia";
import { ref } from "vue";

const THEME_KEY = "taos-viewer:theme";
const SIDEBAR_WIDTH_KEY = "taos-viewer:sidebar-width";

export type ThemeMode = "dark" | "light";

function loadSidebarWidth(): number {
  const v = Number(localStorage.getItem(SIDEBAR_WIDTH_KEY));
  return Number.isFinite(v) && v >= 180 && v <= 520 ? v : 250;
}

export const useUiStore = defineStore("ui", () => {
  const theme = ref<ThemeMode>(
    (localStorage.getItem(THEME_KEY) as ThemeMode) || "dark"
  );

  const sidebarWidth = ref<number>(loadSidebarWidth());

  function toggleTheme() {
    theme.value = theme.value === "dark" ? "light" : "dark";
    localStorage.setItem(THEME_KEY, theme.value);
  }

  function setSidebarWidth(w: number) {
    const clamped = Math.min(520, Math.max(180, Math.round(w)));
    sidebarWidth.value = clamped;
    localStorage.setItem(SIDEBAR_WIDTH_KEY, String(clamped));
  }

  return { theme, toggleTheme, sidebarWidth, setSidebarWidth };
});
