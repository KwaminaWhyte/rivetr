import { redirect } from "react-router";
import type { Route } from "./+types/logout";

export async function action({ request }: Route.ActionArgs) {
  const { logout } = await import("@/lib/session.server");
  return logout(request);
}

export async function loader() {
  return redirect("/login");
}
