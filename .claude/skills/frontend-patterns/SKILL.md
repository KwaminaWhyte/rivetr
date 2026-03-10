---
name: frontend-patterns
description: React Router v7, React Query, and shadcn/ui patterns used in the Rivetr dashboard. Use when writing new frontend components, routes, or API integrations.
allowed-tools: Read, Grep, Glob
---

# Frontend Patterns for Rivetr

## Stack

React Router v7 (Framework/SSR mode) + Vite + TypeScript + React Query v5 + shadcn/ui + Tailwind CSS v4

## Route Structure

Routes defined in `frontend/app/routes.ts` using `@react-router/dev/routes`:

```typescript
import { route, layout, index } from "@react-router/dev/routes";

export default [
  route("login", "routes/login.tsx"),
  layout("routes/_layout.tsx", [
    index("routes/_index.tsx"),
    route("apps/:id", "routes/apps/$id/_layout.tsx", [
      index("routes/apps/$id/_index.tsx"),
      route("settings", "routes/apps/$id/settings.tsx"),
    ]),
  ]),
]
```

## API Client Pattern

All API calls go through `frontend/app/lib/api/core.ts`:

```typescript
// Domain-specific modules export named API objects
export const appsApi = {
  getApps: (options?: { teamId?: string }, token?: string) =>
    apiRequest<App[]>("/apps", { teamId: options?.teamId }, token),

  createApp: (data: CreateAppRequest, token?: string) =>
    apiRequest<App>("/apps", { method: "POST", body: JSON.stringify(data) }, token),

  deleteApp: (id: string, password: string, token?: string) =>
    apiRequest<void>(`/apps/${id}`, { method: "DELETE", body: JSON.stringify({ password }) }, token),
};
```

New API modules go in `frontend/app/lib/api/` and are re-exported from `index.ts`.

## React Query - Data Fetching

```typescript
const { data: app, isLoading } = useQuery<App>({
  queryKey: ["app", id],
  queryFn: () => appsApi.getApp(id!),
  enabled: !!id,
});

// Dynamic refetch interval based on state
const { data: deploymentsData } = useQuery<DeploymentListResponse>({
  queryKey: ["deployments", id],
  queryFn: () => appsApi.getDeployments(id!, { per_page: 20 }),
  enabled: !!id,
  refetchInterval: (query) => {
    const data = query.state.data;
    const hasActive = data?.items.some(d => isActiveDeployment(d.status));
    return hasActive ? 2000 : 30000;
  },
});
```

## React Query - Mutations

```typescript
const createMutation = useMutation({
  mutationFn: (data: CreateRequest) => api.create(data),
  onSuccess: () => {
    toast.success("Created successfully");
    queryClient.invalidateQueries({ queryKey: ["items"] });
    setShowDialog(false);
  },
  onError: (error) => {
    toast.error(error instanceof Error ? error.message : "Failed to create");
  },
});

// In JSX
<Button disabled={createMutation.isPending} onClick={() => createMutation.mutate(formData)}>
  Create
</Button>
```

## Team Context

Multi-tenant support via `useTeamContext()` from `frontend/app/lib/team-context.tsx`:

```typescript
const { currentTeamId, currentTeam, teams } = useTeamContext();

// Pass team ID to queries that are team-scoped
const { data: apps } = useQuery<App[]>({
  queryKey: ["apps", currentTeamId],
  queryFn: () => appsApi.getApps({ teamId: currentTeamId ?? undefined }),
  enabled: currentTeamId !== null,
});
```

Team switching invalidates dependent queries (`apps`, `projects`, `databases`) automatically.

## Auth Pattern

Token-based auth via `frontend/app/lib/auth.ts`:

- `useRequireAuth()` - protects routes, redirects to `/login`
- `usePublicRoute()` - redirects authenticated users to dashboard
- Token stored in localStorage, injected via `apiRequest()` automatically

## Page Component Pattern

```typescript
export default function SettingsPage() {
  const queryClient = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  const { data: items = [], isLoading } = useQuery<Item[]>({
    queryKey: ["items"],
    queryFn: () => itemsApi.getItems(),
  });

  if (isLoading) return <LoadingSkeleton />;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Page Title</h1>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add Item
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Items</CardTitle>
        </CardHeader>
        <CardContent>
          {items.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">No items found.</p>
          ) : (
            <Table>
              <TableHeader>...</TableHeader>
              <TableBody>
                {items.map(item => <TableRow key={item.id}>...</TableRow>)}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <form onSubmit={handleSubmit}>
            <DialogHeader><DialogTitle>Create Item</DialogTitle></DialogHeader>
            <div className="space-y-4 py-4">...</div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowCreateDialog(false)}>Cancel</Button>
              <Button type="submit" disabled={mutation.isPending}>Create</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
```

## Types

All API types in `frontend/app/types/api.ts`. Match backend DTOs:

```typescript
export type AppEnvironment = "development" | "staging" | "production";

export interface App {
  id: string;
  name: string;
  git_url: string;
  // ... fields match Rust serialization
}

export interface CreateAppRequest {
  name: string;
  git_url: string;
  // ... required fields for creation
}

export interface UpdateAppRequest {
  name?: string;       // Optional fields for partial updates
  git_url?: string;
}
```

## UI Conventions

- **Icons**: Lucide React (`lucide-react`) for all icons
- **Toasts**: `sonner` - `toast.success()`, `toast.error()`
- **Loading**: Skeleton components or `isLoading` checks
- **Empty states**: Centered `text-muted-foreground` message
- **Destructive actions**: Confirmation dialog with password input
- **Forms in dialogs**: `<form onSubmit>` wrapping dialog content
- **Status badges**: `<Badge variant="default|secondary|destructive">`
- **Spacing**: `space-y-6` for page sections, `space-y-4` for form fields, `gap-4` for grids
