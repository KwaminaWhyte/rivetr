import { useState, useEffect, useRef } from "react";
import { useNavigate, Link, useSearchParams } from "react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldSeparator,
} from "@/components/ui/field";
import { Rocket, Github, Loader2, ShieldCheck } from "lucide-react";
import { validateAuth, checkSetupStatus, setAuthToken } from "@/lib/auth";
import { oauthApi, type OAuthProviderPublic } from "@/lib/api/oauth";
import { twoFactorApi } from "@/lib/api/two-factor";

// Google icon SVG component
const GoogleIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24">
    <path
      d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 01-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"
      fill="#4285F4"
    />
    <path
      d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
      fill="#34A853"
    />
    <path
      d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
      fill="#FBBC05"
    />
    <path
      d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
      fill="#EA4335"
    />
  </svg>
);

// GitHub icon SVG for consistency
const GitHubIcon = ({ className }: { className?: string }) => (
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" className={className}>
    <path
      d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"
      fill="currentColor"
    />
  </svg>
);

export function meta() {
  return [
    { title: "Login - Rivetr" },
    { name: "description", content: "Sign in to your Rivetr deployment platform" },
  ];
}

export default function LoginPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [oauthProviders, setOauthProviders] = useState<OAuthProviderPublic[]>([]);
  const [oauthLoading, setOauthLoading] = useState<string | null>(null);

  // 2FA state
  const [requires2FA, setRequires2FA] = useState(false);
  const [tempToken, setTempToken] = useState<string | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [useRecoveryCode, setUseRecoveryCode] = useState(false);
  const [is2FASubmitting, setIs2FASubmitting] = useState(false);
  const totpInputRef = useRef<HTMLInputElement>(null);

  // Check for OAuth callback token in URL params
  useEffect(() => {
    const oauthToken = searchParams.get("oauth_token");
    if (oauthToken) {
      setAuthToken(oauthToken);
      // Clean the URL and redirect
      const returnUrl = localStorage.getItem("rivetr_return_url");
      if (returnUrl) {
        localStorage.removeItem("rivetr_return_url");
        navigate(returnUrl, { replace: true });
      } else {
        navigate("/", { replace: true });
      }
    }
  }, [searchParams, navigate]);

  // Check auth status and load OAuth providers on mount
  useEffect(() => {
    async function checkAuth() {
      // Check if setup is needed
      const needsSetup = await checkSetupStatus();
      if (needsSetup) {
        navigate("/setup", { replace: true });
        return;
      }

      // Check if already logged in
      const isAuthenticated = await validateAuth();
      if (isAuthenticated) {
        navigate("/", { replace: true });
        return;
      }

      // Load enabled OAuth providers
      try {
        const providers = await oauthApi.getEnabledProviders();
        setOauthProviders(providers);
      } catch {
        // OAuth providers are optional, don't fail the page
      }

      setIsLoading(false);
    }
    checkAuth();
  }, [navigate]);

  // Focus TOTP input when 2FA form shows
  useEffect(() => {
    if (requires2FA && totpInputRef.current) {
      totpInputRef.current.focus();
    }
  }, [requires2FA, useRecoveryCode]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSubmitting(true);
    setError(null);

    const formData = new FormData(event.currentTarget);
    const email = formData.get("email") as string;
    const password = formData.get("password") as string;

    if (!email || !password) {
      setError("Email and password are required");
      setIsSubmitting(false);
      return;
    }

    try {
      const response = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
        credentials: "include",
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || "Invalid email or password");
      }

      const data = await response.json();

      // Check if 2FA is required
      if (data.requires_2fa) {
        setTempToken(data.token);
        setRequires2FA(true);
        setIsSubmitting(false);
        return;
      }

      // No 2FA - complete login
      setAuthToken(data.token);

      // Check for return URL (e.g., from invitation accept flow)
      const returnUrl = localStorage.getItem("rivetr_return_url");
      if (returnUrl) {
        localStorage.removeItem("rivetr_return_url");
        navigate(returnUrl, { replace: true });
      } else {
        navigate("/", { replace: true });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
      setIsSubmitting(false);
    }
  }

  async function handle2FASubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!tempToken || !totpCode.trim()) return;

    setIs2FASubmitting(true);
    setError(null);

    try {
      const data = await twoFactorApi.validate(tempToken, totpCode.trim());
      setAuthToken(data.token);

      const returnUrl = localStorage.getItem("rivetr_return_url");
      if (returnUrl) {
        localStorage.removeItem("rivetr_return_url");
        navigate(returnUrl, { replace: true });
      } else {
        navigate("/", { replace: true });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Verification failed");
      setTotpCode("");
      setIs2FASubmitting(false);
    }
  }

  // Auto-submit when 6 digits entered (TOTP mode only)
  function handleTotpChange(value: string) {
    // Only allow digits in TOTP mode
    if (!useRecoveryCode) {
      value = value.replace(/\D/g, "").slice(0, 6);
    }
    setTotpCode(value);
  }

  async function handleOAuthLogin(provider: string) {
    setOauthLoading(provider);
    setError(null);

    try {
      const { authorization_url } = await oauthApi.getLoginAuthorizeUrl(provider);
      // Redirect the browser to the OAuth provider
      window.location.href = authorization_url;
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to start ${provider} login`);
      setOauthLoading(null);
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-pulse text-muted-foreground">Loading...</div>
      </div>
    );
  }

  const hasOAuthProviders = oauthProviders.length > 0;
  const githubEnabled = oauthProviders.some((p) => p.provider === "github");
  const googleEnabled = oauthProviders.some((p) => p.provider === "google");

  return (
    <div className="grid min-h-svh lg:grid-cols-2">
      <div className="flex flex-col gap-4 p-6 md:p-10">
        <div className="flex justify-center gap-2 md:justify-start">
          <Link to="/" className="flex items-center gap-2 font-medium">
            <div className="bg-primary text-primary-foreground flex size-6 items-center justify-center rounded-md">
              <Rocket className="size-4" />
            </div>
            Rivetr
          </Link>
        </div>
        <div className="flex flex-1 items-center justify-center">
          <div className="w-full max-w-xs">
            {requires2FA ? (
              /* 2FA Verification Form */
              <form onSubmit={handle2FASubmit} className="flex flex-col gap-6">
                <FieldGroup>
                  <div className="flex flex-col items-center gap-2 text-center">
                    <div className="flex size-12 items-center justify-center rounded-full bg-primary/10">
                      <ShieldCheck className="size-6 text-primary" />
                    </div>
                    <h1 className="text-2xl font-bold">Two-Factor Authentication</h1>
                    <p className="text-muted-foreground text-sm text-balance">
                      {useRecoveryCode
                        ? "Enter one of your recovery codes"
                        : "Enter the 6-digit code from your authenticator app"}
                    </p>
                  </div>
                  {error && (
                    <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm text-center">
                      {error}
                    </div>
                  )}
                  <Field>
                    <FieldLabel htmlFor="totp-code">
                      {useRecoveryCode ? "Recovery Code" : "Authentication Code"}
                    </FieldLabel>
                    <Input
                      ref={totpInputRef}
                      id="totp-code"
                      type="text"
                      inputMode={useRecoveryCode ? "text" : "numeric"}
                      autoComplete="one-time-code"
                      placeholder={useRecoveryCode ? "ABCD1234" : "000000"}
                      value={totpCode}
                      onChange={(e) => handleTotpChange(e.target.value)}
                      className={useRecoveryCode ? "" : "text-center text-2xl tracking-[0.5em] font-mono"}
                      maxLength={useRecoveryCode ? 8 : 6}
                      required
                    />
                  </Field>
                  <Field>
                    <Button
                      type="submit"
                      className="w-full"
                      disabled={is2FASubmitting || (!useRecoveryCode && totpCode.length !== 6)}
                    >
                      {is2FASubmitting ? "Verifying..." : "Verify"}
                    </Button>
                  </Field>
                  <div className="text-center">
                    <button
                      type="button"
                      className="text-sm text-muted-foreground hover:text-foreground underline-offset-4 hover:underline"
                      onClick={() => {
                        setUseRecoveryCode(!useRecoveryCode);
                        setTotpCode("");
                        setError(null);
                      }}
                    >
                      {useRecoveryCode
                        ? "Use authenticator app instead"
                        : "Use a recovery code"}
                    </button>
                  </div>
                  <div className="text-center">
                    <button
                      type="button"
                      className="text-sm text-muted-foreground hover:text-foreground underline-offset-4 hover:underline"
                      onClick={() => {
                        setRequires2FA(false);
                        setTempToken(null);
                        setTotpCode("");
                        setError(null);
                        setUseRecoveryCode(false);
                      }}
                    >
                      Back to login
                    </button>
                  </div>
                </FieldGroup>
              </form>
            ) : (
              /* Normal Login Form */
              <form onSubmit={handleSubmit} className="flex flex-col gap-6">
                <FieldGroup>
                  <div className="flex flex-col items-center gap-1 text-center">
                    <h1 className="text-2xl font-bold">Login to your account</h1>
                    <p className="text-muted-foreground text-sm text-balance">
                      Enter your credentials to access the dashboard
                    </p>
                  </div>
                  {error && (
                    <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm text-center">
                      {error}
                    </div>
                  )}

                  {/* OAuth buttons - show when providers are enabled */}
                  {hasOAuthProviders && (
                    <>
                      <div className="flex flex-col gap-2">
                        {githubEnabled && (
                          <Button
                            variant="outline"
                            type="button"
                            className="w-full gap-2"
                            disabled={oauthLoading !== null}
                            onClick={() => handleOAuthLogin("github")}
                          >
                            {oauthLoading === "github" ? (
                              <Loader2 className="size-4 animate-spin" />
                            ) : (
                              <GitHubIcon className="size-4" />
                            )}
                            Continue with GitHub
                          </Button>
                        )}
                        {googleEnabled && (
                          <Button
                            variant="outline"
                            type="button"
                            className="w-full gap-2"
                            disabled={oauthLoading !== null}
                            onClick={() => handleOAuthLogin("google")}
                          >
                            {oauthLoading === "google" ? (
                              <Loader2 className="size-4 animate-spin" />
                            ) : (
                              <GoogleIcon className="size-4" />
                            )}
                            Continue with Google
                          </Button>
                        )}
                      </div>
                      <FieldSeparator>Or continue with email</FieldSeparator>
                    </>
                  )}

                  <Field>
                    <FieldLabel htmlFor="email">Email</FieldLabel>
                    <Input
                      id="email"
                      name="email"
                      type="email"
                      placeholder="admin@example.com"
                      required
                      autoFocus={!hasOAuthProviders}
                    />
                  </Field>
                  <Field>
                    <div className="flex items-center">
                      <FieldLabel htmlFor="password">Password</FieldLabel>
                    </div>
                    <Input
                      id="password"
                      name="password"
                      type="password"
                      required
                    />
                  </Field>
                  <Field>
                    <Button type="submit" className="w-full" disabled={isSubmitting}>
                      {isSubmitting ? "Signing in..." : "Login"}
                    </Button>
                  </Field>
                </FieldGroup>
              </form>
            )}
          </div>
        </div>
      </div>
      <div className="bg-muted relative hidden lg:block">
        <div className="absolute inset-0 flex items-center justify-center bg-gradient-to-br from-primary/20 to-primary/5">
          <div className="text-center p-8">
            <Rocket className="size-24 mx-auto mb-6 text-primary/50" />
            <h2 className="text-3xl font-bold mb-2">Rivetr</h2>
            <p className="text-muted-foreground">
              Deploy applications with ease
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
