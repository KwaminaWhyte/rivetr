import { TeamPage } from "@/components/teams/team-detail";

export function meta() {
  return [
    { title: "Team audit log - Rivetr" },
    { name: "description", content: "Recent activity in the current team" },
  ];
}

export default function TeamSectionPage() {
  return <TeamPage section="audit" />;
}
