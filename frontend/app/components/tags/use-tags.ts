import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { tagsApi, type TagAssignment, type TaggableKind, type TagWithCount } from "@/lib/api/tags";

/** All tags visible in the current team context. */
export function useTags(teamId?: string | null) {
  return useQuery<TagWithCount[]>({
    queryKey: ["tags", teamId ?? null],
    queryFn: () => tagsApi.list(teamId),
    staleTime: 30000,
  });
}

/** Tag assignments for the team, as a lookup `kind:id -> Set<tagId>`. */
export function useTagAssignments(teamId?: string | null, enabled = true) {
  const query = useQuery<TagAssignment[]>({
    queryKey: ["tag-assignments", teamId ?? null],
    queryFn: () => tagsApi.assignments({ teamId }),
    staleTime: 30000,
    enabled,
  });
  const byResource = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const a of query.data ?? []) {
      const key = `${a.resource_type}:${a.resource_id}`;
      let set = map.get(key);
      if (!set) {
        set = new Set();
        map.set(key, set);
      }
      set.add(a.tag_id);
    }
    return map;
  }, [query.data]);

  /** Does the resource carry `tagId`? A null tag means "no filter" and always matches. */
  const hasTag = useMemo(
    () => (kind: TaggableKind, id: string, tagId: string | null) =>
      tagId === null || (byResource.get(`${kind}:${id}`)?.has(tagId) ?? false),
    [byResource]
  );

  return { ...query, byResource, hasTag };
}
