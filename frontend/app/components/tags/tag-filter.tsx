import { Tag as TagIcon } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { TagWithCount } from "@/lib/api/tags";
import { cn } from "@/lib/utils";

const ALL = "__all_tags__";

/** "All tags" or one tag. Renders nothing while there are no tags. */
export function TagFilter({
  tags,
  value,
  onChange,
  className,
  size,
}: {
  tags: TagWithCount[];
  value: string | null;
  onChange: (tagId: string | null) => void;
  className?: string;
  size?: "sm" | "default";
}) {
  if (tags.length === 0) return null;
  return (
    <Select value={value ?? ALL} onValueChange={(v) => onChange(v === ALL ? null : v)}>
      <SelectTrigger size={size} aria-label="Filter by tag" className={cn("w-40", className)}>
        <TagIcon className="size-3.5 text-muted-foreground" aria-hidden="true" />
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={ALL}>All tags</SelectItem>
        {tags.map((t) => (
          <SelectItem key={t.id} value={t.id}>
            <span
              aria-hidden="true"
              className="size-2 shrink-0 rounded-full"
              style={{ backgroundColor: t.color }}
            />
            {t.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
