import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { ChevronRight, KeyRound, Palette, ShieldCheck, SlidersHorizontal } from "lucide-react";

import { QueryError } from "@/components/query-error";
import { DetailField } from "@/components/metrics";
import { PageHeader, SectionSkeleton } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { getCurrentUser } from "@/lib/auth";

export function meta() {
  return [
    { title: "Account - Rivetr" },
    { name: "description", content: "Your profile, security and dashboard preferences" },
  ];
}

const LINKS = [
  { to: "/account/security", icon: ShieldCheck, title: "Security", text: "Password and two-factor authentication." },
  { to: "/account/appearance", icon: Palette, title: "Appearance", text: "Theme and colours of the dashboard." },
  { to: "/account/preferences", icon: SlidersHorizontal, title: "Preferences", text: "Dates, log viewer, notifications and density." },
  { to: "/keys/api-tokens", icon: KeyRound, title: "API tokens", text: "Personal tokens for the API and CLI." },
];

export default function AccountProfilePage() {
  const { data: user, isLoading, error, refetch } = useQuery({
    queryKey: ["current-user"],
    queryFn: getCurrentUser,
    staleTime: 5 * 60 * 1000,
  });

  return (
    <div className="space-y-6">
      <PageHeader title="Profile" description="The account you are signed in with." />

      {error && !user ? (
        <QueryError what="your profile" error={error} onRetry={() => refetch()} />
      ) : isLoading || !user ? (
        <SectionSkeleton rows={2} />
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>{user.name}</CardTitle>
            <CardDescription>{user.email}</CardDescription>
          </CardHeader>
          <CardContent>
            <dl className="grid gap-x-8 gap-y-5 sm:grid-cols-3">
              <DetailField label="Role">
                <Badge variant="outline" className="capitalize">{user.role}</Badge>
              </DetailField>
              <DetailField label="Two-factor authentication">
                {user.totp_enabled ? "Enabled" : "Not enabled"}
              </DetailField>
              <DetailField label="User ID" mono>
                {user.id}
              </DetailField>
            </dl>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-3 sm:grid-cols-2">
        {LINKS.map((l) => (
          <Link
            key={l.to}
            to={l.to}
            className="group flex items-center gap-3 rounded-lg border bg-card p-4 transition-colors hover:bg-muted/50"
          >
            <l.icon className="size-5 shrink-0 text-muted-foreground" />
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium">{l.title}</span>
              <span className="block text-xs text-muted-foreground">{l.text}</span>
            </span>
            <ChevronRight className="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
          </Link>
        ))}
      </div>
    </div>
  );
}
