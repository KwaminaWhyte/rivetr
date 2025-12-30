import { createCookieSessionStorage, redirect } from "react-router";

const SESSION_SECRET =
  process.env.SESSION_SECRET || "dev-secret-change-in-production";
const API_BASE = process.env.API_BASE || "http://localhost:8080";

type SessionData = {
  token: string;
};

type SessionFlashData = {
  error: string;
};

export const sessionStorage = createCookieSessionStorage<
  SessionData,
  SessionFlashData
>({
  cookie: {
    name: "__rivetr_session",
    httpOnly: true,
    path: "/",
    sameSite: "lax",
    secrets: [SESSION_SECRET],
    secure: process.env.NODE_ENV === "production",
    maxAge: 60 * 60 * 24 * 7, // 7 days
  },
});

export async function getSession(request: Request) {
  const cookie = request.headers.get("Cookie");
  return sessionStorage.getSession(cookie);
}

export async function createUserSession(token: string, redirectTo: string) {
  const session = await sessionStorage.getSession();
  session.set("token", token);
  return redirect(redirectTo, {
    headers: {
      "Set-Cookie": await sessionStorage.commitSession(session),
    },
  });
}

export async function logout(request: Request) {
  const session = await getSession(request);
  return redirect("/login", {
    headers: {
      "Set-Cookie": await sessionStorage.destroySession(session),
    },
  });
}

export async function getToken(request: Request): Promise<string | null> {
  const session = await getSession(request);
  return session.get("token") || null;
}

export async function requireAuth(request: Request): Promise<string> {
  const session = await getSession(request);
  const token = session.get("token");

  if (!token) {
    throw redirect("/login");
  }

  // Validate token with backend
  try {
    const response = await fetch(`${API_BASE}/api/auth/validate`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (!response.ok) {
      throw redirect("/login");
    }
  } catch (error) {
    if (error instanceof Response) throw error;
    throw redirect("/login");
  }

  return token;
}

export async function checkSetupStatus(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE}/api/auth/setup-status`);
    const data = await response.json();
    return data.needs_setup;
  } catch {
    return false;
  }
}
