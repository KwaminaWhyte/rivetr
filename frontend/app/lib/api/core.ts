/**
 * Core API utilities shared across all API modules.
 * Contains the base request function and authentication helpers.
 */

/**
 * Get the stored authentication token from localStorage.
 */
export function getStoredToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("rivetr_auth_token");
}

/**
 * Make an authenticated API request.
 * @param path - API path (without /api prefix)
 * @param options - Fetch options
 * @param token - Optional auth token (uses stored token if not provided)
 */
export async function apiRequest<T>(
  path: string,
  options: RequestInit = {},
  token?: string
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };

  // Add Authorization header - use provided token or get from localStorage
  const authToken = token || getStoredToken();
  if (authToken) {
    headers["Authorization"] = `Bearer ${authToken}`;
  }

  const response = await fetch(`/api${path}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || `API error: ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

/**
 * Make an authenticated request that returns a raw Response (for file downloads, etc.)
 */
export async function apiRawRequest(
  path: string,
  options: RequestInit = {},
  token?: string
): Promise<Response> {
  const headers: Record<string, string> = {
    ...(options.headers as Record<string, string>),
  };

  const authToken = token || getStoredToken();
  if (authToken) {
    headers["Authorization"] = `Bearer ${authToken}`;
  }

  const response = await fetch(`/api${path}`, {
    ...options,
    headers,
    credentials: "include",
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || `API error: ${response.status}`);
  }

  return response;
}
