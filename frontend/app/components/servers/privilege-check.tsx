/**
 * Result of the "root or passwordless sudo" server validation check, with the
 * exact sudoers line to add when sudo is missing.
 */
import { Copy, ShieldAlert, ShieldCheck } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { PrivilegeCheck } from "@/lib/api/servers";

export function PrivilegeCheckBadge({ check }: { check: PrivilegeCheck }) {
  const label =
    check.status === "root"
      ? "Root user"
      : check.status === "available"
        ? "Passwordless sudo"
        : "Sudo missing";
  return (
    <Badge
      variant={check.ok ? "outline" : "destructive"}
      className="text-xs gap-1"
      title={check.message}
    >
      {check.ok ? <ShieldCheck className="h-3 w-3" /> : <ShieldAlert className="h-3 w-3" />}
      {label}
    </Badge>
  );
}

export function PrivilegeCheckFix({ check }: { check: PrivilegeCheck }) {
  if (check.ok || !check.sudoers_line) return null;
  const line = check.sudoers_line;
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(line);
      toast.success("Sudoers line copied");
    } catch {
      toast.error("Could not copy to clipboard");
    }
  };
  return (
    <div className="mb-1.5 max-w-md space-y-1 rounded-md border border-destructive/30 bg-destructive/5 p-2 text-left text-xs">
      <p>
        {check.status === "not_installed"
          ? `Install sudo, then run sudo visudo -f /etc/sudoers.d/rivetr and add:`
          : `Run sudo visudo -f /etc/sudoers.d/rivetr on the server and add:`}
      </p>
      <div className="flex items-center gap-1">
        <code className="flex-1 truncate rounded bg-muted px-1.5 py-0.5 font-mono">{line}</code>
        <Button type="button" variant="ghost" size="sm" className="h-6 px-1.5" onClick={copy}>
          <Copy className="h-3 w-3" />
          <span className="sr-only">Copy sudoers line</span>
        </Button>
      </div>
    </div>
  );
}
