/**
 * Advanced Docker Swarm service spec for an app: placement constraints and
 * spread preferences, update and rollback config, resource reservations and
 * restart policy. Only shown when the Docker host is in an active swarm or the
 * app already has Swarm services.
 */
import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SectionCard, FormGrid, FormField } from "@/components/page";
import {
  swarmApi,
  type SwarmRolloutConfig,
  type SwarmSpecConfig,
} from "@/lib/api/swarm";

const NONE = "__default";

function lines(text: string): string[] {
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
}

function num(v: string): number | null {
  if (v.trim() === "") return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

function str(v: number | null | undefined): string {
  return v === null || v === undefined ? "" : String(v);
}

interface RolloutState {
  parallelism: string;
  delay: string;
  failureAction: string;
  order: string;
  monitor: string;
  maxFailureRatio: string;
}

function rolloutFrom(cfg?: SwarmRolloutConfig | null): RolloutState {
  return {
    parallelism: str(cfg?.parallelism),
    delay: str(cfg?.delay_secs),
    failureAction: cfg?.failure_action ?? NONE,
    order: cfg?.order ?? NONE,
    monitor: str(cfg?.monitor_secs),
    maxFailureRatio: str(cfg?.max_failure_ratio),
  };
}

function rolloutTo(s: RolloutState): SwarmRolloutConfig | null {
  const cfg: SwarmRolloutConfig = {
    parallelism: num(s.parallelism),
    delay_secs: num(s.delay),
    failure_action:
      s.failureAction === NONE ? null : (s.failureAction as SwarmRolloutConfig["failure_action"]),
    order: s.order === NONE ? null : (s.order as SwarmRolloutConfig["order"]),
    monitor_secs: num(s.monitor),
    max_failure_ratio: num(s.maxFailureRatio),
  };
  return Object.values(cfg).some((v) => v !== null) ? cfg : null;
}

function RolloutFields({
  prefix,
  value,
  onChange,
  allowRollback,
}: {
  prefix: string;
  value: RolloutState;
  onChange: (v: RolloutState) => void;
  allowRollback: boolean;
}) {
  const set = (patch: Partial<RolloutState>) => onChange({ ...value, ...patch });
  return (
    <FormGrid columns={3}>
      <FormField label="Parallelism" htmlFor={`${prefix}-par`} hint="Tasks at once, 0 = all">
        <Input
          id={`${prefix}-par`}
          type="number"
          min={0}
          value={value.parallelism}
          onChange={(e) => set({ parallelism: e.target.value })}
          placeholder="1"
        />
      </FormField>
      <FormField label="Delay (seconds)" htmlFor={`${prefix}-delay`}>
        <Input
          id={`${prefix}-delay`}
          type="number"
          min={0}
          value={value.delay}
          onChange={(e) => set({ delay: e.target.value })}
          placeholder="0"
        />
      </FormField>
      <FormField label="Monitor (seconds)" htmlFor={`${prefix}-monitor`}>
        <Input
          id={`${prefix}-monitor`}
          type="number"
          min={0}
          value={value.monitor}
          onChange={(e) => set({ monitor: e.target.value })}
          placeholder="5"
        />
      </FormField>
      <FormField label="Failure action">
        <Select value={value.failureAction} onValueChange={(v) => set({ failureAction: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NONE}>Docker default (pause)</SelectItem>
            <SelectItem value="pause">Pause</SelectItem>
            <SelectItem value="continue">Continue</SelectItem>
            {allowRollback && <SelectItem value="rollback">Roll back</SelectItem>}
          </SelectContent>
        </Select>
      </FormField>
      <FormField label="Order">
        <Select value={value.order} onValueChange={(v) => set({ order: v })}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NONE}>Docker default (stop-first)</SelectItem>
            <SelectItem value="stop-first">Stop first</SelectItem>
            <SelectItem value="start-first">Start first</SelectItem>
          </SelectContent>
        </Select>
      </FormField>
      <FormField label="Max failure ratio" htmlFor={`${prefix}-ratio`} hint="0 to 1">
        <Input
          id={`${prefix}-ratio`}
          type="number"
          min={0}
          max={1}
          step={0.05}
          value={value.maxFailureRatio}
          onChange={(e) => set({ maxFailureRatio: e.target.value })}
          placeholder="0"
        />
      </FormField>
    </FormGrid>
  );
}

export function SwarmSpecCard({ appId }: { appId: string }) {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ["app-swarm-spec", appId],
    queryFn: () => swarmApi.getAppSwarmSpec(appId),
  });

  const [constraints, setConstraints] = useState("");
  const [spread, setSpread] = useState("");
  const [maxPerNode, setMaxPerNode] = useState("");
  const [reserveCpus, setReserveCpus] = useState("");
  const [reserveMem, setReserveMem] = useState("");
  const [restartCondition, setRestartCondition] = useState(NONE);
  const [restartDelay, setRestartDelay] = useState("");
  const [restartAttempts, setRestartAttempts] = useState("");
  const [restartWindow, setRestartWindow] = useState("");
  const [update, setUpdate] = useState<RolloutState>(rolloutFrom());
  const [rollback, setRollback] = useState<RolloutState>(rolloutFrom());

  useEffect(() => {
    if (!data) return;
    const s = data.spec;
    setConstraints(s.constraints.join("\n"));
    setSpread(s.spread_preferences.join("\n"));
    setMaxPerNode(str(s.max_replicas_per_node));
    setReserveCpus(str(s.reserve_cpus));
    setReserveMem(str(s.reserve_memory_mb));
    setRestartCondition(s.restart_policy?.condition ?? NONE);
    setRestartDelay(str(s.restart_policy?.delay_secs));
    setRestartAttempts(str(s.restart_policy?.max_attempts));
    setRestartWindow(str(s.restart_policy?.window_secs));
    setUpdate(rolloutFrom(s.update_config));
    setRollback(rolloutFrom(s.rollback_config));
  }, [data]);

  const save = useMutation({
    mutationFn: () => {
      const spec: SwarmSpecConfig = {
        constraints: lines(constraints),
        spread_preferences: lines(spread),
        max_replicas_per_node: num(maxPerNode),
        reserve_cpus: num(reserveCpus),
        reserve_memory_mb: num(reserveMem),
        restart_policy:
          restartCondition === NONE
            ? null
            : {
                condition: restartCondition as "none" | "on-failure" | "any",
                delay_secs: num(restartDelay),
                max_attempts: num(restartAttempts),
                window_secs: num(restartWindow),
              },
        update_config: rolloutTo(update),
        rollback_config: rolloutTo(rollback),
      };
      return swarmApi.updateAppSwarmSpec(appId, spec);
    },
    onSuccess: (res) => {
      queryClient.invalidateQueries({ queryKey: ["app-swarm-spec", appId] });
      if (res.errors.length > 0) {
        toast.error(`Saved, but some services were not updated: ${res.errors.join("; ")}`);
      } else if (res.applied.length > 0) {
        toast.success(`Swarm spec saved and applied to ${res.applied.join(", ")}`);
      } else {
        toast.success("Swarm spec saved. It applies when a Swarm service is created.");
      }
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save Swarm spec"),
  });

  if (isLoading || !data) return null;
  // Only relevant for apps that target Swarm.
  if (!data.swarm_active && data.services.length === 0) return null;

  return (
    <SectionCard
      id="swarm"
      title="Docker Swarm service"
      description="Placement, rolling updates, reservations and restart policy for this app's Swarm services."
      footer={
        <Button onClick={() => save.mutate()} disabled={save.isPending}>
          {save.isPending ? "Saving..." : "Save Swarm spec"}
        </Button>
      }
    >
      <div className="space-y-6">
        <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
          {data.services.length > 0 ? (
            <>
              Applies to
              {data.services.map((s) => (
                <Badge key={s.id} variant="outline">
                  {s.service_name}
                </Badge>
              ))}
            </>
          ) : (
            <span>
              No Swarm service is bound to this app yet. The spec is used when one is created
              from the Swarm page.
            </span>
          )}
        </div>

        <div className="space-y-3">
          <h4 className="text-sm font-medium">Placement</h4>
          <FormGrid>
            <FormField
              label="Constraints"
              htmlFor="swarm-constraints"
              hint="One per line, e.g. node.role==worker or node.labels.zone!=eu-1"
            >
              <Textarea
                id="swarm-constraints"
                rows={3}
                className="font-mono text-sm"
                value={constraints}
                onChange={(e) => setConstraints(e.target.value)}
                placeholder="node.role==worker"
              />
            </FormField>
            <FormField
              label="Spread preferences"
              htmlFor="swarm-spread"
              hint="One node attribute per line; tasks are spread evenly across its values"
            >
              <Textarea
                id="swarm-spread"
                rows={3}
                className="font-mono text-sm"
                value={spread}
                onChange={(e) => setSpread(e.target.value)}
                placeholder="node.labels.zone"
              />
            </FormField>
            <FormField label="Max replicas per node" htmlFor="swarm-max-per-node">
              <Input
                id="swarm-max-per-node"
                type="number"
                min={0}
                value={maxPerNode}
                onChange={(e) => setMaxPerNode(e.target.value)}
                placeholder="Unlimited"
              />
            </FormField>
          </FormGrid>
        </div>

        <div className="space-y-3">
          <h4 className="text-sm font-medium">Resource reservations</h4>
          <FormGrid>
            <FormField label="CPU (cores)" htmlFor="swarm-reserve-cpu">
              <Input
                id="swarm-reserve-cpu"
                type="number"
                min={0}
                step={0.1}
                value={reserveCpus}
                onChange={(e) => setReserveCpus(e.target.value)}
                placeholder="0.5"
              />
            </FormField>
            <FormField label="Memory (MB)" htmlFor="swarm-reserve-mem">
              <Input
                id="swarm-reserve-mem"
                type="number"
                min={0}
                value={reserveMem}
                onChange={(e) => setReserveMem(e.target.value)}
                placeholder="256"
              />
            </FormField>
          </FormGrid>
        </div>

        <div className="space-y-3">
          <h4 className="text-sm font-medium">Restart policy</h4>
          <FormGrid columns={3}>
            <FormField label="Condition">
              <Select value={restartCondition} onValueChange={setRestartCondition}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={NONE}>Docker default (any)</SelectItem>
                  <SelectItem value="any">Any</SelectItem>
                  <SelectItem value="on-failure">On failure</SelectItem>
                  <SelectItem value="none">None</SelectItem>
                </SelectContent>
              </Select>
            </FormField>
            <FormField label="Delay (seconds)" htmlFor="swarm-restart-delay">
              <Input
                id="swarm-restart-delay"
                type="number"
                min={0}
                disabled={restartCondition === NONE}
                value={restartDelay}
                onChange={(e) => setRestartDelay(e.target.value)}
              />
            </FormField>
            <FormField label="Max attempts" htmlFor="swarm-restart-attempts" hint="0 = unlimited">
              <Input
                id="swarm-restart-attempts"
                type="number"
                min={0}
                disabled={restartCondition === NONE}
                value={restartAttempts}
                onChange={(e) => setRestartAttempts(e.target.value)}
              />
            </FormField>
            <FormField label="Window (seconds)" htmlFor="swarm-restart-window">
              <Input
                id="swarm-restart-window"
                type="number"
                min={0}
                disabled={restartCondition === NONE}
                value={restartWindow}
                onChange={(e) => setRestartWindow(e.target.value)}
              />
            </FormField>
          </FormGrid>
        </div>

        <div className="space-y-3">
          <h4 className="text-sm font-medium">Update config</h4>
          <RolloutFields prefix="swarm-update" value={update} onChange={setUpdate} allowRollback />
        </div>

        <div className="space-y-3">
          <h4 className="text-sm font-medium">Rollback config</h4>
          <RolloutFields
            prefix="swarm-rollback"
            value={rollback}
            onChange={setRollback}
            allowRollback={false}
          />
        </div>
      </div>
    </SectionCard>
  );
}
