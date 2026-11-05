import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { KeyRound, Pencil, Plus, Trash2 } from "lucide-react";
import { EmptyState, SectionCard, relativeTime } from "@/components/page";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  passkeyErrorMessage,
  passkeysApi,
  passkeysSupported,
  type PasskeyInfo,
} from "@/lib/api/passkeys";

function defaultName(): string {
  if (typeof navigator === "undefined") return "Passkey";
  const ua = navigator.userAgent;
  if (/iPhone|iPad/.test(ua)) return "iPhone or iPad";
  if (/Mac OS X/.test(ua)) return "Mac";
  if (/Android/.test(ua)) return "Android";
  if (/Windows/.test(ua)) return "Windows";
  return "Passkey";
}

/** Passkeys section of the account security page: list, add, rename, delete. */
export function PasskeysCard() {
  const queryClient = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  const [renaming, setRenaming] = useState<PasskeyInfo | null>(null);
  const [deleting, setDeleting] = useState<PasskeyInfo | null>(null);

  const { data: status } = useQuery({
    queryKey: ["passkeys-status"],
    queryFn: passkeysApi.status,
  });
  const { data: passkeys = [], isLoading } = useQuery({
    queryKey: ["passkeys"],
    queryFn: passkeysApi.list,
  });
  const enabled = status?.enabled ?? true;
  const supported = passkeysSupported();

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["passkeys"] });

  const register = useMutation({
    mutationFn: (n: string) => passkeysApi.register(n),
    onSuccess: () => {
      toast.success("Passkey added");
      setAdding(false);
      invalidate();
    },
    onError: (e) => toast.error(passkeyErrorMessage(e)),
  });
  const rename = useMutation({
    mutationFn: (v: { id: string; name: string }) => passkeysApi.rename(v.id, v.name),
    onSuccess: () => {
      setRenaming(null);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });
  const remove = useMutation({
    mutationFn: (id: string) => passkeysApi.remove(id),
    onSuccess: () => {
      toast.success("Passkey removed");
      setDeleting(null);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  return (
    <SectionCard
      id="passkeys"
      title="Passkeys"
      description="Sign in with your fingerprint, face or device PIN instead of a password. A passkey sign in also counts as two-factor authentication."
      actions={
        enabled && supported ? (
          <Button
            size="sm"
            onClick={() => {
              setName(defaultName());
              setAdding(true);
            }}
          >
            <Plus className="size-4" />
            Add passkey
          </Button>
        ) : undefined
      }
      flush
    >
      {!enabled && (
        <p className="border-b px-4 py-3 text-sm text-muted-foreground">
          Passkeys are turned off for this instance. An admin can turn them on in Settings &gt; Advanced.
        </p>
      )}
      {enabled && !supported && (
        <p className="border-b px-4 py-3 text-sm text-muted-foreground">
          This browser does not support passkeys.
        </p>
      )}
      {isLoading ? (
        <p className="px-4 py-6 text-sm text-muted-foreground">Loading...</p>
      ) : passkeys.length === 0 ? (
        <EmptyState
          compact
          icon={<KeyRound />}
          title="No passkeys yet"
          description="Add one on each device you sign in from."
        />
      ) : (
        <ul className="divide-y">
          {passkeys.map((p) => (
            <li key={p.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
              <KeyRound className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{p.name}</p>
                <p className="text-xs text-muted-foreground">
                  Added {relativeTime(p.created_at)} · {p.last_used_at ? `last used ${relativeTime(p.last_used_at)}` : "never used"}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={`Rename ${p.name}`}
                onClick={() => {
                  setName(p.name);
                  setRenaming(p);
                }}
              >
                <Pencil />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={`Delete ${p.name}`}
                onClick={() => setDeleting(p)}
              >
                <Trash2 />
              </Button>
            </li>
          ))}
        </ul>
      )}

      <Dialog
        open={adding || !!renaming}
        onOpenChange={(o) => {
          if (!o) {
            setAdding(false);
            setRenaming(null);
          }
        }}
      >
        <DialogContent>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (renaming) rename.mutate({ id: renaming.id, name: name.trim() });
              else register.mutate(name.trim());
            }}
          >
            <DialogHeader>
              <DialogTitle>{renaming ? "Rename passkey" : "Add a passkey"}</DialogTitle>
              <DialogDescription>
                {renaming
                  ? "Pick a name that tells you which device this is."
                  : "Name it after the device, then follow your browser's prompt."}
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-2 py-4">
              <Label htmlFor="passkey-name">Name</Label>
              <Input
                id="passkey-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                maxLength={64}
                autoFocus
              />
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setAdding(false);
                  setRenaming(null);
                }}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={register.isPending || rename.isPending}>
                {renaming ? "Save" : register.isPending ? "Waiting for passkey..." : "Continue"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(o) => !o && setDeleting(null)}
        title={`Delete passkey "${deleting?.name ?? ""}"?`}
        description="You will no longer be able to sign in with it. Remove it from the device's password manager too."
        isPending={remove.isPending}
        onConfirm={() => {
          if (deleting) remove.mutate(deleting.id);
        }}
      />
    </SectionCard>
  );
}
