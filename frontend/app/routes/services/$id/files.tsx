import { useEffect, useState } from "react";
import { useOutletContext } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { ContainerFileBrowser } from "@/components/container-file-browser";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import type { Service } from "@/types/api";

export function meta() {
  return [
    { title: "Files - Rivetr" },
    { name: "description", content: "Browse files inside a service container" },
  ];
}

export default function ServiceFilesTab() {
  const { service } = useOutletContext<{ service: Service }>();
  const { data: containers = [], isLoading } = useQuery({
    queryKey: ["service-containers", service.id],
    queryFn: () => api.getServiceContainers(service.id),
  });
  const running = containers.filter((c) => c.state === "running");
  const [container, setContainer] = useState<string>("");

  // Default to the first running container, and drop a choice that stopped.
  useEffect(() => {
    if (!running.some((c) => c.name === container)) {
      setContainer(running[0]?.name ?? "");
    }
  }, [running, container]);

  const unavailable = isLoading
    ? "Loading containers..."
    : running.length === 0
      ? "Start the service to browse its files."
      : null;

  return (
    <ContainerFileBrowser
      key={container}
      target={{ kind: "service", id: service.id, container }}
      unavailableMessage={unavailable}
      toolbarExtra={
        running.length > 0 ? (
          <Select value={container} onValueChange={setContainer}>
            <SelectTrigger aria-label="Container" className="w-full lg:w-56">
              <SelectValue placeholder="Container" />
            </SelectTrigger>
            <SelectContent>
              {running.map((c) => (
                <SelectItem key={c.id} value={c.name}>
                  {c.service || c.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : null
      }
    />
  );
}
