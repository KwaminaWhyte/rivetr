import { TeamPage } from "@/components/teams/team-detail";

export function meta() {
  return [
    { title: "Team roles - Rivetr" },
    { name: "description", content: "Role presets and custom roles of the current team" },
  ];
}

export default function TeamSectionPage() {
  return <TeamPage section="roles" />;
}
