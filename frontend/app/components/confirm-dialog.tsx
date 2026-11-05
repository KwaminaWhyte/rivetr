import { useEffect, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";

import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { instanceSettingsApi } from "@/lib/api/instance-settings";
import type { ConfirmationMode } from "@/types/instance-settings";

/**
 * The instance-wide confirmation mode (Settings > Advanced). Falls back to
 * "simple" while loading or when the request fails.
 */
export function useConfirmationMode(): ConfirmationMode {
  const { data } = useQuery({
    queryKey: ["destructive-confirmation"],
    queryFn: () => instanceSettingsApi.getConfirmationMode(),
    staleTime: 5 * 60 * 1000,
  });
  return data?.mode ?? "simple";
}

export interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  /**
   * Name of the thing being changed (app, database, server...). In two-step
   * mode the user has to type it before the action is enabled.
   */
  resourceName?: string;
  confirmLabel?: string;
  /** Destructive styling for the confirm button (default true). */
  destructive?: boolean;
  /** Force a mode instead of the instance setting. */
  mode?: ConfirmationMode;
  isPending?: boolean;
  onConfirm: () => void | Promise<void>;
}

/**
 * Shared confirm dialog for destructive actions. In "two_step" mode (set by
 * an admin in Settings > Advanced) the user must type `resourceName` first.
 */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  resourceName,
  confirmLabel = "Delete",
  destructive = true,
  mode,
  isPending = false,
  onConfirm,
}: ConfirmDialogProps) {
  const instanceMode = useConfirmationMode();
  const effectiveMode = mode ?? instanceMode;
  const twoStep = effectiveMode === "two_step" && !!resourceName;
  const [typed, setTyped] = useState("");

  useEffect(() => {
    if (!open) setTyped("");
  }, [open]);

  const canConfirm = !isPending && (!twoStep || typed.trim() === resourceName);

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          {description && <AlertDialogDescription>{description}</AlertDialogDescription>}
        </AlertDialogHeader>
        {twoStep && (
          <div className="space-y-2">
            <Label htmlFor="confirm-resource-name" className="text-sm font-normal">
              Type <span className="font-mono font-semibold">{resourceName}</span> to confirm
            </Label>
            <Input
              id="confirm-resource-name"
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              autoComplete="off"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && canConfirm) {
                  e.preventDefault();
                  void onConfirm();
                }
              }}
            />
          </div>
        )}
        <AlertDialogFooter>
          <AlertDialogCancel disabled={isPending}>Cancel</AlertDialogCancel>
          <Button
            variant={destructive ? "destructive" : "default"}
            disabled={!canConfirm}
            onClick={() => void onConfirm()}
          >
            {isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {confirmLabel}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
