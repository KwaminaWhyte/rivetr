import { RivetrBrand } from "@/components/rivetr-logo";
import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { setAuthToken, validateAuth } from "@/lib/auth";
import { instanceSettingsApi } from "@/lib/api/instance-settings";

// Matches server-side validation (min 12 characters)
const registerSchema = z
  .object({
    name: z.string().trim().min(1, "Name is required"),
    email: z.string().trim().email("Enter a valid email"),
    password: z.string().min(12, "Password must be at least 12 characters"),
    confirmPassword: z.string(),
  })
  .refine((d) => d.password === d.confirmPassword, {
    message: "Passwords do not match",
    path: ["confirmPassword"],
  });

export function meta() {
  return [
    { title: "Create account - Rivetr" },
    { name: "description", content: "Create a Rivetr account" },
  ];
}

export default function RegisterPage() {
  const navigate = useNavigate();
  const [status, setStatus] = useState<"loading" | "open" | "closed">("loading");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function check() {
      if (await validateAuth()) {
        navigate("/", { replace: true });
        return;
      }
      try {
        const r = await instanceSettingsApi.getRegistrationStatus();
        setStatus(r.password_registration ? "open" : "closed");
      } catch {
        setStatus("closed");
      }
    }
    check();
  }, [navigate]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    const form = new FormData(event.currentTarget);
    const parsed = registerSchema.safeParse({
      name: form.get("name"),
      email: form.get("email"),
      password: form.get("password"),
      confirmPassword: form.get("confirmPassword"),
    });
    if (!parsed.success) {
      setError(parsed.error.issues[0]?.message ?? "Check the form");
      return;
    }
    setIsSubmitting(true);
    try {
      const res = await instanceSettingsApi.register({
        name: parsed.data.name,
        email: parsed.data.email,
        password: parsed.data.password,
      });
      setAuthToken(res.token);
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Sign up failed");
      setIsSubmitting(false);
    }
  }

  return (
    <div className="flex min-h-svh flex-col gap-4 p-6 md:p-10">
      <div className="flex justify-center gap-2 md:justify-start">
        <Link to="/" className="flex items-center gap-2 font-medium">
          <RivetrBrand logoClassName="size-7" />
        </Link>
      </div>
      <div className="flex flex-1 items-center justify-center">
        <div className="w-full max-w-xs">
          {status === "loading" ? (
            <div className="animate-pulse text-center text-muted-foreground">Loading...</div>
          ) : status === "closed" ? (
            <div className="flex flex-col gap-4 text-center">
              <h1 className="text-2xl font-bold">Registration is closed</h1>
              <p className="text-sm text-muted-foreground">
                New accounts on this instance are created by invitation. Ask an admin to invite you.
              </p>
              <Button asChild variant="outline">
                <Link to="/login">Back to login</Link>
              </Button>
            </div>
          ) : (
            <form onSubmit={handleSubmit} className="flex flex-col gap-6">
              <FieldGroup>
                <div className="flex flex-col items-center gap-1 text-center">
                  <h1 className="text-2xl font-bold">Create an account</h1>
                  <p className="text-sm text-balance text-muted-foreground">
                    You get your own team after signing up
                  </p>
                </div>
                {error && (
                  <div className="rounded-md bg-destructive/10 p-3 text-center text-sm text-destructive">
                    {error}
                  </div>
                )}
                <Field>
                  <FieldLabel htmlFor="name">Name</FieldLabel>
                  <Input id="name" name="name" autoComplete="name" required autoFocus />
                </Field>
                <Field>
                  <FieldLabel htmlFor="email">Email</FieldLabel>
                  <Input id="email" name="email" type="email" autoComplete="email" required />
                </Field>
                <Field>
                  <FieldLabel htmlFor="password">Password</FieldLabel>
                  <Input
                    id="password"
                    name="password"
                    type="password"
                    autoComplete="new-password"
                    required
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="confirmPassword">Confirm password</FieldLabel>
                  <Input
                    id="confirmPassword"
                    name="confirmPassword"
                    type="password"
                    autoComplete="new-password"
                    required
                  />
                </Field>
                <Field>
                  <Button type="submit" className="w-full" disabled={isSubmitting}>
                    {isSubmitting ? "Creating account..." : "Create account"}
                  </Button>
                </Field>
                <p className="text-center text-sm text-muted-foreground">
                  Already have an account?{" "}
                  <Link to="/login" className="text-foreground underline-offset-4 hover:underline">
                    Log in
                  </Link>
                </p>
              </FieldGroup>
            </form>
          )}
        </div>
      </div>
    </div>
  );
}
