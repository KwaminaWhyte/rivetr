import { TeamPage } from "@/components/teams/team-detail";

export function meta() {
  return [
    { title: "Team - Rivetr" },
    { name: "description", content: "Current team settings and notification channels" },
  ];
}

export default function TeamSectionPage() {
  return <TeamPage section="general" />;
}
