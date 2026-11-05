import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Copy, Eye, EyeOff, Loader2, Radar } from "lucide-react";

import { FormGrid, SettingsSection } from "@/components/settings-section";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { instanceSettingsApi } from "@/lib/api/instance-settings";
import { useIsInstanceAdmin } from "@/lib/auth";

function IpField({
  id,
  label,
  value,
  onChange,
  placeholder,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
}) {
  const [shown, setShown] = useState(false);
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex gap-1">
        <Input
          id={id}
          type={shown ? "text" : "password"}
          autoComplete="off"
          className="font-mono"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
        />
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label={shown ? `Hide ${label}` : `Show ${label}`}
          onClick={() => setShown((s) => !s)}
        >
          {shown ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
        </Button>
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label={`Copy ${label}`}
          disabled={!value}
          onClick={() => {
            navigator.clipboard
              .writeText(value)
              .then(() => toast.success(`${label} copied`))
              .catch(() => toast.error("Copy failed"));
          }}
        >
          <Copy className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

/** Public IPv4 / IPv6 of this server, used as the target for domain checks. */
export function PublicIpCard() {
  return useIsInstanceAdmin() ? <PublicIpCardInner /> : null;
}

function PublicIpCardInner() {
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: ["public-ips"],
    queryFn: () => instanceSettingsApi.getPublicIps(),
  });
  const [ipv4, setIpv4] = useState("");
  const [ipv6, setIpv6] = useState("");

  useEffect(() => {
    if (data) {
      setIpv4(data.ipv4 ?? "");
      setIpv6(data.ipv6 ?? "");
    }
  }, [data]);

  const dirty = !!data && (ipv4 !== (data.ipv4 ?? "") || ipv6 !== (data.ipv6 ?? ""));

  const detect = useMutation({
    mutationFn: () => instanceSettingsApi.detectPublicIps(),
    onSuccess: (r) => {
      if (r.ipv4) setIpv4(r.ipv4);
      if (r.ipv6) setIpv6(r.ipv6);
      if (!r.ipv4 && !r.ipv6) toast.error("Could not detect a public IP");
      else toast.success("Detected. Save to keep the values.");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Detection failed"),
  });

  const save = useMutation({
    mutationFn: () =>
      instanceSettingsApi.updatePublicIps({ ipv4: ipv4.trim(), ipv6: ipv6.trim() }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["public-ips"], saved);
      toast.success("Public IPs saved");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save"),
  });

  return (
    <SettingsSection
      title="Public IP"
      description="Where your domains should point. Used by domain DNS checks."
      actions={
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => detect.mutate()}
          disabled={detect.isPending}
        >
          {detect.isPending ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Radar className="mr-2 h-4 w-4" />
          )}
          Detect
        </Button>
      }
    >
      <div className="space-y-4">
        <FormGrid>
          <IpField id="public-ipv4" label="IPv4" value={ipv4} onChange={setIpv4} placeholder="203.0.113.10" />
          <IpField id="public-ipv6" label="IPv6" value={ipv6} onChange={setIpv6} placeholder="2001:db8::10" />
        </FormGrid>
        <div className="flex justify-end">
          <Button type="button" size="sm" onClick={() => save.mutate()} disabled={!dirty || save.isPending}>
            {save.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save
          </Button>
        </div>
      </div>
    </SettingsSection>
  );
}
