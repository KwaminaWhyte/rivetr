import { Link } from "react-router";
import { Compass } from "lucide-react";

import { EmptyState } from "@/components/page";
import { Button } from "@/components/ui/button";

export function meta() {
  return [{ title: "Page not found - Rivetr" }];
}

/** Dashboard 404: unknown URLs render inside the shell, with breadcrumbs. */
export default function NotFoundPage() {
  return (
    <EmptyState
      bordered
      icon={<Compass />}
      title="Page not found"
      description="This page does not exist or has moved. Use the sidebar or search (Cmd+K) to find what you need."
      action={
        <Button asChild>
          <Link to="/">Back to dashboard</Link>
        </Button>
      }
    />
  );
}
