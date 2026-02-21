import type { ApiResponse } from "../../types/api";

const apiBaseUrl = import.meta.env.VITE_API_BASE_URL as string | undefined;
if (!apiBaseUrl) {
  throw new Error(
    "VITE_API_BASE_URL is required. Define it in your env profile (e.g. web/.env.development).",
  );
}

try {
  // Enforce absolute URL so browser/dev/proxy contexts resolve consistently.
  new URL(apiBaseUrl);
} catch {
  throw new Error(
    `VITE_API_BASE_URL must be an absolute URL, got: ${apiBaseUrl}`,
  );
}

const apiKey = import.meta.env.VITE_API_KEY as string | undefined;

export function withQuery(
  path: string,
  query?: Record<string, string | number | undefined>,
): string {
  if (!query) {
    return path;
  }

  const params = new URLSearchParams();

  Object.entries(query).forEach(([key, value]) => {
    if (value !== undefined && value !== "") {
      params.set(key, String(value));
    }
  });

  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
}

export async function apiGet<T>(
  path: string,
  query?: Record<string, string | number | undefined>,
): Promise<T> {
  const url = `${apiBaseUrl}${withQuery(path, query)}`;

  const headers = new Headers({
    Accept: "application/json",
  });

  if (apiKey) {
    headers.set("Authorization", `Bearer ${apiKey}`);
  }

  const response = await fetch(url, {
    method: "GET",
    headers,
  });

  if (!response.ok) {
    throw new Error(`Request failed (${response.status}) for ${path}`);
  }

  const body = (await response.json()) as ApiResponse<T>;
  return body.data;
}
