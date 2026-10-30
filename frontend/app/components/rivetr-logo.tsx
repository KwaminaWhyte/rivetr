import { cn } from "@/lib/utils";
import { useWhiteLabel } from "@/lib/white-label-context";

export function RivetrLogo({
  className,
  decorative = false,
}: {
  className?: string;
  decorative?: boolean;
}) {
  const { config } = useWhiteLabel();

  return (
    <img
      src={config?.logo_url || "/brand/rivetr-mark.png"}
      alt={decorative ? "" : config?.app_name || "Rivetr"}
      aria-hidden={decorative || undefined}
      width={32}
      height={32}
      className={cn("size-8 shrink-0 object-contain", className)}
    />
  );
}

export function RivetrBrand({
  className,
  logoClassName,
}: {
  className?: string;
  logoClassName?: string;
}) {
  const { config } = useWhiteLabel();

  return (
    <span className={cn("flex items-center gap-2 font-semibold", className)}>
      <RivetrLogo decorative className={logoClassName} />
      <span className="truncate">{config?.app_name || "Rivetr"}</span>
    </span>
  );
}
