import { useState, useEffect } from "react";
import { useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Rocket, CheckCircle } from "lucide-react";
import { validateAuth, checkSetupStatus } from "@/lib/auth";

export function meta() {
  return [
    { title: "Setup - Rivetr" },
    { name: "description", content: "Set up your Rivetr deployment platform" },
  ];
}

export default function SetupPage() {
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Check if setup is needed on mount
  useEffect(() => {
    async function checkAuth() {
      const needsSetup = await checkSetupStatus();
      if (!needsSetup) {
        // Setup already done, check if logged in
        const isAuthenticated = await validateAuth();
        if (isAuthenticated) {
          navigate("/", { replace: true });
        } else {
          navigate("/login", { replace: true });
        }
        return;
      }

      setIsLoading(false);
    }
    checkAuth();
  }, [navigate]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSubmitting(true);
    setError(null);

    const formData = new FormData(event.currentTarget);
    const name = formData.get("name") as string;
    const email = formData.get("email") as string;
    const password = formData.get("password") as string;
    const confirmPassword = formData.get("confirmPassword") as string;

    // Validation
    if (!name || !email || !password) {
      setError("All fields are required");
      setIsSubmitting(false);
      return;
    }

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      setIsSubmitting(false);
      return;
    }

    if (password.length < 8) {
      setError("Password must be at least 8 characters");
      setIsSubmitting(false);
      return;
    }

    try {
      const response = await fetch("/api/auth/setup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, email, password }),
        credentials: "include",
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || "Setup failed");
      }

      // Successful setup - navigate to dashboard
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Setup failed");
      setIsSubmitting(false);
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-pulse text-muted-foreground">Loading...</div>
      </div>
    );
  }

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
            <form onSubmit={handleSubmit} className="flex flex-col gap-6">
              <FieldGroup>
                <div className="flex flex-col items-center gap-1 text-center">
                  <h1 className="text-2xl font-bold">Welcome to Rivetr</h1>
                  <p className="text-muted-foreground text-sm text-balance">
                    Create your admin account to get started
                  </p>
                </div>
                {error && (
                  <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm text-center">
                    {error}
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
            </form>
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
