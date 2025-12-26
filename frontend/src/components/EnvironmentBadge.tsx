import { Badge } from "@/components/ui/badge";
import type { AppEnvironment } from "@/types/api";

interface EnvironmentBadgeProps {
  environment: AppEnvironment;
  className?: string;
}

const environmentConfig: Record<
  AppEnvironment,
  { label: string; className: string }
> = {
  development: {
    label: "Development",
    className: "bg-gray-500 hover:bg-gray-600 text-white",
  },
  staging: {
    label: "Staging",
    className: "bg-yellow-500 hover:bg-yellow-600 text-white",
  },
  production: {
    label: "Production",
    className: "bg-green-500 hover:bg-green-600 text-white",
  },
};

export function EnvironmentBadge({
  environment,
  className = "",
}: EnvironmentBadgeProps) {
  const config = environmentConfig[environment];

  return (
    <Badge className={`${config.className} ${className}`}>{config.label}</Badge>
  );
}
