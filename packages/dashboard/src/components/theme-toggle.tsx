"use client";

import { useTheme } from "next-themes";
import { Moon, Sun } from "lucide-react";
import { cn } from "@/lib/utils";

export function ThemeToggle({ className, showLabel = false }: { className?: string; showLabel?: boolean }) {
  const { resolvedTheme, setTheme } = useTheme();
  const nextTheme = resolvedTheme === "dark" ? "light" : "dark";

  return (
    <button
      type="button"
      onClick={() => setTheme(nextTheme)}
      className={cn(
        "flex items-center justify-center gap-2 rounded text-muted-foreground hover:bg-muted hover:text-foreground",
        showLabel ? "w-full px-1.5 py-1 text-left text-sm" : "size-5",
        className,
      )}
      title={`Switch to ${nextTheme} mode`}
    >
      {resolvedTheme === "dark" ? (
        <Sun className="size-3" />
      ) : (
        <Moon className="size-3" />
      )}
      {showLabel && <span className="flex-1">Switch to {nextTheme} mode</span>}
    </button>
  );
}
