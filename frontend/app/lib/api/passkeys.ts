/**
 * Passkeys (E2): WebAuthn registration and sign in.
 *
 * The server speaks the webauthn-rs JSON shapes, where binary fields are
 * base64url strings. The browser API wants ArrayBuffers, so options are
 * decoded before `navigator.credentials.*` and responses encoded after.
 */

import { apiRequest } from "./core";

export interface PasskeyInfo {
  id: string;
  name: string;
  created_at: string;
  last_used_at: string | null;
}

export interface PasskeyLoginResult {
  token: string;
  user: { id: string; email: string; name: string; role: string; totp_enabled: boolean };
}

function b64urlToBuffer(value: string): ArrayBuffer {
  const pad = "=".repeat((4 - (value.length % 4)) % 4);
  const b64 = (value + pad).replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(b64);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out.buffer;
}

function bufferToB64url(buf: ArrayBuffer | null | undefined): string | undefined {
  if (!buf) return undefined;
  const bytes = new Uint8Array(buf);
  let s = "";
  for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** True when this browser can create and use passkeys at all. */
export function passkeysSupported(): boolean {
  return typeof window !== "undefined" && !!window.PublicKeyCredential && !!navigator.credentials;
}

// Loosely typed: the server JSON is converted field by field.
/* eslint-disable @typescript-eslint/no-explicit-any */
function decodeCreationOptions(options: any): PublicKeyCredentialCreationOptions {
  const pk = options.publicKey;
  return {
    ...pk,
    challenge: b64urlToBuffer(pk.challenge),
    user: { ...pk.user, id: b64urlToBuffer(pk.user.id) },
    excludeCredentials: (pk.excludeCredentials ?? []).map((c: any) => ({
      ...c,
      id: b64urlToBuffer(c.id),
    })),
  };
}

function decodeRequestOptions(options: any): PublicKeyCredentialRequestOptions {
  const pk = options.publicKey;
  return {
    ...pk,
    challenge: b64urlToBuffer(pk.challenge),
    allowCredentials: (pk.allowCredentials ?? []).map((c: any) => ({
      ...c,
      id: b64urlToBuffer(c.id),
    })),
  };
}
/* eslint-enable @typescript-eslint/no-explicit-any */

function encodeRegistration(cred: PublicKeyCredential) {
  const r = cred.response as AuthenticatorAttestationResponse;
  return {
    id: cred.id,
    rawId: bufferToB64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: bufferToB64url(r.attestationObject),
      clientDataJSON: bufferToB64url(r.clientDataJSON),
      transports: typeof r.getTransports === "function" ? r.getTransports() : undefined,
    },
    extensions: cred.getClientExtensionResults(),
  };
}

function encodeAssertion(cred: PublicKeyCredential) {
  const r = cred.response as AuthenticatorAssertionResponse;
  return {
    id: cred.id,
    rawId: bufferToB64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: bufferToB64url(r.authenticatorData),
      clientDataJSON: bufferToB64url(r.clientDataJSON),
      signature: bufferToB64url(r.signature),
      userHandle: bufferToB64url(r.userHandle),
    },
    extensions: cred.getClientExtensionResults(),
  };
}

export const passkeysApi = {
  status: () => apiRequest<{ enabled: boolean }>("/auth/passkeys/status"),

  list: () => apiRequest<PasskeyInfo[]>("/passkeys"),

  rename: (id: string, name: string) =>
    apiRequest<void>(`/passkeys/${id}`, { method: "PUT", body: JSON.stringify({ name }) }),

  remove: (id: string) => apiRequest<void>(`/passkeys/${id}`, { method: "DELETE" }),

  /** Run the full registration ceremony and store the passkey under `name`. */
  register: async (name: string): Promise<PasskeyInfo> => {
    const start = await apiRequest<{ challenge_id: string; options: unknown }>(
      "/passkeys/register/start",
      { method: "POST" }
    );
    const cred = (await navigator.credentials.create({
      publicKey: decodeCreationOptions(start.options),
    })) as PublicKeyCredential | null;
    if (!cred) throw new Error("No passkey was created");
    return apiRequest<PasskeyInfo>("/passkeys/register/finish", {
      method: "POST",
      body: JSON.stringify({
        challenge_id: start.challenge_id,
        name,
        credential: encodeRegistration(cred),
      }),
    });
  },

  /** Sign in with any passkey registered for this instance (no email needed). */
  login: async (): Promise<PasskeyLoginResult> => {
    const start = await apiRequest<{ challenge_id: string; options: unknown }>(
      "/auth/passkey/login/start",
      { method: "POST" }
    );
    const cred = (await navigator.credentials.get({
      publicKey: decodeRequestOptions(start.options),
    })) as PublicKeyCredential | null;
    if (!cred) throw new Error("No passkey was selected");
    return apiRequest<PasskeyLoginResult>("/auth/passkey/login/finish", {
      method: "POST",
      body: JSON.stringify({ challenge_id: start.challenge_id, credential: encodeAssertion(cred) }),
    });
  },
};

/** Turn a WebAuthn DOMException into a short message for a toast. */
export function passkeyErrorMessage(e: unknown): string {
  if (e instanceof DOMException) {
    if (e.name === "NotAllowedError") return "The passkey prompt was cancelled or timed out.";
    if (e.name === "InvalidStateError") return "This device already has a passkey for your account.";
    if (e.name === "SecurityError")
      return "Passkeys need the dashboard to be opened through its domain (or localhost).";
  }
  return e instanceof Error ? e.message : "Passkey request failed";
}
