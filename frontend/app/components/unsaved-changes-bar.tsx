import { useEffect } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface UnsavedChangesBarProps {
  /** Show the bar (usually `form.isDirty`). */
  show: boolean;
  onSave: () => void;
  onReset: () => void;
  saving?: boolean;
  /** Disable Save, e.g. while the form is invalid. */
  saveDisabled?: boolean;
  message?: string;
}

/** Elements where Enter already means something else. */
function enterIsTaken(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "BUTTON" || tag === "A" || tag === "SELECT") return true;
  const role = target.getAttribute("role");
  if (role === "combobox" || role === "option" || role === "menuitem" || role === "button") {
    return true;
  }
  // Inside an open dialog or menu, Enter belongs to it.
  return Boolean(target.closest("[role='dialog'], [role='alertdialog'], [role='menu'], [role='listbox']"));
}

/**
 * Sticky bar shown while a settings form has unsaved edits. Enter saves (outside
 * text areas, buttons and dialogs); Reset discards the edits.
 */
export function UnsavedChangesBar({
  show,
  onSave,
  onReset,
  saving = false,
  saveDisabled = false,
  message = "You have changes that haven't been saved yet.",
}: UnsavedChangesBarProps) {
  useEffect(() => {
    if (!show) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Enter" || e.shiftKey || e.altKey || e.isComposing) return;
      if (e.defaultPrevented || enterIsTaken(e.target)) return;
      if (saving || saveDisabled) return;
      e.preventDefault();
      onSave();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [show, onSave, saving, saveDisabled]);

  return (
    <div
      aria-hidden={!show}
      className={cn(
        "pointer-events-none fixed inset-x-0 bottom-4 z-40 flex justify-center px-4 transition-all duration-200",
        show ? "translate-y-0 opacity-100" : "translate-y-4 opacity-0"
      )}
    >
      <div
        role="status"
        aria-live="polite"
        className={cn(
          "flex w-full max-w-2xl flex-col gap-3 rounded-lg border bg-background/95 p-3 shadow-lg backdrop-blur sm:flex-row sm:items-center sm:justify-between",
          show && "pointer-events-auto"
        )}
      >
        <p className="text-sm font-medium">{message}</p>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onReset}
            disabled={saving || !show}
            tabIndex={show ? 0 : -1}
          >
            Reset
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={onSave}
            disabled={saving || saveDisabled || !show}
            tabIndex={show ? 0 : -1}
            className="gap-2"
          >
            {saving && <Loader2 className="h-4 w-4 animate-spin" />}
            Save changes
            <kbd className="hidden rounded border border-primary-foreground/30 px-1 text-[10px] font-normal sm:inline">
              Enter
            </kbd>
          </Button>
        </div>
      </div>
    </div>
  );
}
