import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ServiceDeployHook } from "@/types/api";
import { Plus, Trash2 } from "lucide-react";

interface Props {
  title: string;
  description: React.ReactNode;
  hooks: ServiceDeployHook[];
  onChange: (hooks: ServiceDeployHook[]) => void;
  composeServices: string[];
  placeholder?: string;
}

/** Editor for a list of pre- or post-deploy hooks. */
export function ServiceHooksEditor({
  title,
  description,
  hooks,
  onChange,
  composeServices,
  placeholder,
}: Props) {
  const update = (i: number, patch: Partial<ServiceDeployHook>) =>
    onChange(hooks.map((h, idx) => (idx === i ? { ...h, ...patch } : h)));

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {hooks.length === 0 && (
          <p className="text-sm text-muted-foreground">No hooks.</p>
        )}
        {hooks.map((hook, i) => (
          <div key={i} className="grid gap-3 rounded-lg border p-4 sm:grid-cols-[1fr_1fr_2fr_auto_auto]">
            <div className="grid gap-1.5">
              <Label>Name</Label>
              <Input
                value={hook.name ?? ""}
                onChange={(e) => update(i, { name: e.target.value || undefined })}
                placeholder="migrate"
              />
            </div>
            <div className="grid gap-1.5">
              <Label>Run in</Label>
              <Select value={hook.service} onValueChange={(v) => update(i, { service: v })}>
                <SelectTrigger>
                  <SelectValue placeholder="Compose service" />
                </SelectTrigger>
                <SelectContent>
                  {composeServices.map((s) => (
                    <SelectItem key={s} value={s}>
                      {s}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1.5">
              <Label>Command</Label>
              <Input
                className="font-mono text-xs"
                value={hook.command}
                onChange={(e) => update(i, { command: e.target.value })}
                placeholder={placeholder}
              />
            </div>
            <div className="grid gap-1.5">
              <Label>Timeout (s)</Label>
              <Input
                type="number"
                min={1}
                className="w-24"
                value={hook.timeout_secs ?? ""}
                onChange={(e) =>
                  update(i, {
                    timeout_secs: e.target.value ? Number(e.target.value) : undefined,
                  })
                }
                placeholder="600"
              />
            </div>
            <div className="flex items-end">
              <Button
                variant="ghost"
                size="sm"
                aria-label="Remove hook"
                onClick={() => onChange(hooks.filter((_, idx) => idx !== i))}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          </div>
        ))}
        <Button
          variant="outline"
          size="sm"
          className="gap-2"
          onClick={() =>
            onChange([...hooks, { service: composeServices[0] ?? "", command: "" }])
          }
        >
          <Plus className="h-4 w-4" />
          Add hook
        </Button>
      </CardContent>
    </Card>
  );
}
