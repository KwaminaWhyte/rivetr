import { Form, redirect, useNavigation } from "react-router";
import type { Route } from "./+types/setup";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Rocket, CheckCircle } from "lucide-react";

export function meta() {
  return [
    { title: "Setup - Rivetr" },
    { name: "description", content: "Set up your Rivetr deployment platform" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { getToken, checkSetupStatus } = await import("@/lib/session.server");

  // Check if setup is needed
  const needsSetup = await checkSetupStatus();
  if (!needsSetup) {
    // Setup already done, check if logged in
    const token = await getToken(request);
    if (token) {
      throw redirect("/");
    }
    throw redirect("/login");
  }

  return null;
}

export async function action({ request }: Route.ActionArgs) {
  const { createUserSession } = await import("@/lib/session.server");
  const { setup } = await import("@/lib/api.server");

  const formData = await request.formData();
  const name = formData.get("name");
  const email = formData.get("email");
  const password = formData.get("password");
  const confirmPassword = formData.get("confirmPassword");

  // Validation
  if (
    typeof name !== "string" ||
    typeof email !== "string" ||
    typeof password !== "string" ||
    typeof confirmPassword !== "string"
  ) {
    return { error: "Invalid form data" };
  }

  if (!name || !email || !password) {
    return { error: "All fields are required" };
  }

  if (password !== confirmPassword) {
    return { error: "Passwords do not match" };
  }

  if (password.length < 8) {
    return { error: "Password must be at least 8 characters" };
  }

  try {
    const result = await setup({ name, email, password });
    return createUserSession(result.token, "/");
  } catch (error) {
    return { error: error instanceof Error ? error.message : "Setup failed" };
  }
}

export default function SetupPage({ actionData }: Route.ComponentProps) {
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="grid min-h-svh lg:grid-cols-2">
      <div className="flex flex-col gap-4 p-6 md:p-10">
        <div className="flex justify-center gap-2 md:justify-start">
          <div className="flex items-center gap-2 font-medium">
            <div className="bg-primary text-primary-foreground flex size-6 items-center justify-center rounded-md">
              <Rocket className="size-4" />
            </div>
            Rivetr
          </div>
        </div>
        <div className="flex flex-1 items-center justify-center">
          <div className="w-full max-w-xs">
            <Form method="post" className="flex flex-col gap-6">
              <FieldGroup>
                <div className="flex flex-col items-center gap-1 text-center">
                  <h1 className="text-2xl font-bold">Welcome to Rivetr</h1>
                  <p className="text-muted-foreground text-sm text-balance">
                    Create your admin account to get started
                  </p>
                </div>
                {actionData?.error && (
                  <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm text-center">
                    {actionData.error}
                  </div>
                )}
                <Field>
                  <FieldLabel htmlFor="name">Your Name</FieldLabel>
                  <Input
                    id="name"
                    name="name"
                    type="text"
                    placeholder="John Doe"
                    required
                    autoFocus
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="email">Email</FieldLabel>
                  <Input
                    id="email"
                    name="email"
                    type="email"
                    placeholder="admin@example.com"
                    required
                  />
                  <FieldDescription>
                    You'll use this to log in
                  </FieldDescription>
                </Field>
                <Field>
                  <FieldLabel htmlFor="password">Password</FieldLabel>
                  <Input
                    id="password"
                    name="password"
                    type="password"
                    placeholder="Min 8 characters"
                    required
                    minLength={8}
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="confirmPassword">Confirm Password</FieldLabel>
                  <Input
                    id="confirmPassword"
                    name="confirmPassword"
                    type="password"
                    required
                  />
                </Field>
                <Field>
                  <Button type="submit" className="w-full" disabled={isSubmitting}>
                    {isSubmitting ? "Creating account..." : "Complete Setup"}
                  </Button>
                </Field>
              </FieldGroup>
            </Form>
          </div>
        </div>
      </div>
      <div className="bg-muted relative hidden lg:block">
        <div className="absolute inset-0 flex items-center justify-center bg-gradient-to-br from-primary/20 to-primary/5">
          <div className="text-center p-8 max-w-md">
            <Rocket className="size-24 mx-auto mb-6 text-primary/50" />
            <h2 className="text-3xl font-bold mb-4">Rivetr</h2>
            <p className="text-muted-foreground mb-6">
              A lightweight deployment engine for your applications
            </p>
            <div className="text-left space-y-3">
              <div className="flex items-center gap-2">
                <CheckCircle className="size-5 text-primary" />
                <span className="text-sm">Deploy from Git webhooks</span>
              </div>
              <div className="flex items-center gap-2">
                <CheckCircle className="size-5 text-primary" />
                <span className="text-sm">Docker & Podman support</span>
              </div>
              <div className="flex items-center gap-2">
                <CheckCircle className="size-5 text-primary" />
                <span className="text-sm">Built-in reverse proxy</span>
              </div>
              <div className="flex items-center gap-2">
                <CheckCircle className="size-5 text-primary" />
                <span className="text-sm">Minimal resource usage</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
