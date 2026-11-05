import { useLocation, useNavigate } from "react-router";

import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  activeResourceSection,
  resourceSections,
  type ResourceMatch,
  type ResourceNavOptions,
} from "@/lib/resource-nav";

interface ResourceSectionProps {
  match: ResourceMatch;
  options?: ResourceNavOptions;
  /** Counts shown next to a section, keyed by section id (e.g. pending approvals). */
  badges?: Record<string, number>;
}

/**
 * Sits between a resource's header and its section content. On narrow
 * screens it is the section menu (the contextual sidebar covers it from md
 * up); on every screen it shows the section title and description, so each
 * section page gets the same header.
 */
export function ResourceSection({ match, options, badges }: ResourceSectionProps) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const sections = resourceSections(match, options);
  const active = activeResourceSection(match, pathname, options);
  const isGeneral = active?.id === "general";

  return (
    <div className="space-y-4">
      <div className="md:hidden">
        <Select value={active?.url} onValueChange={(url) => url && navigate(url)}>
          <SelectTrigger className="w-full" aria-label="Section">
            <SelectValue placeholder="Section" />
          </SelectTrigger>
          <SelectContent>
            {sections.map((section, i) => (
              <SelectGroup key={section.label ?? i}>
                {section.label && <SelectLabel>{section.label}</SelectLabel>}
                {section.items.map((item) => (
                  <SelectItem key={item.url} value={item.url}>
                    {item.title}
                    {!!badges?.[item.id] && (
                      <Badge variant="destructive" className="ml-2 h-4 min-w-4 rounded-full px-1 text-[10px]">
                        {badges[item.id]}
                      </Badge>
                    )}
                  </SelectItem>
                ))}
              </SelectGroup>
            ))}
          </SelectContent>
        </Select>
      </div>
      {active && !isGeneral && (
        <div className="border-b pb-3">
          <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
            {active.title}
            {!!badges?.[active.id] && (
              <Badge variant="destructive" className="h-4 min-w-4 rounded-full px-1 text-[10px]">
                {badges[active.id]}
              </Badge>
            )}
          </h2>
          {active.description && (
            <p className="mt-0.5 text-sm text-muted-foreground">{active.description}</p>
          )}
        </div>
      )}
    </div>
  );
}
