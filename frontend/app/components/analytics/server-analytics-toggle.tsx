import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { analyticsApi } from "@/lib/api/analytics";

/** "Enable traffic analytics" switch for one server (localhost or remote). */
export function ServerAnalyticsToggle({
  serverId,
  enabled,
  disabled,
}: {
  serverId: string;
  enabled: boolean;
  disabled?: boolean;
}) {
  const queryClient = useQueryClient();
  const toggle = useMutation({
    mutationFn: (on: boolean) => analyticsApi.setServerEnabled(serverId, on),
    onSuccess: (res) => {
      toast.success(res.enabled ? "Traffic analytics enabled" : "Traffic analytics disabled");
      queryClient.invalidateQueries({ queryKey: ["analytics"] });
      queryClient.invalidateQueries({ queryKey: ["analytics-settings"] });
    },
    onError: (e: Error) => toast.error(e.message),
  });
  const id = `analytics-toggle-${serverId}`;
  return (
    <div className="flex items-center gap-2">
      <Switch
        id={id}
        checked={enabled}
        disabled={disabled || toggle.isPending}
        onCheckedChange={(v) => toggle.mutate(v)}
      />
      <Label htmlFor={id} className="text-sm font-normal">
        Enable traffic analytics
      </Label>
    </div>
  );
}
