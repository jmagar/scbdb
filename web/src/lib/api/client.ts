import type { ApiResponse } from "../../types/api";

const rawApiBaseUrl = import.meta.env.VITE_API_BASE_URL as string | undefined;
const apiBaseUrl = (rawApiBaseUrl ?? "").replace(/\/+$/, "");

if (apiBaseUrl) {
  const isAbsolute = (() => {
    try {
      new URL(apiBaseUrl);
      return true;
    } catch {
      return false;
    }
  })();
  const isRelative = apiBaseUrl.startsWith("/");
  if (!isAbsolute && !isRelative) {
    throw new Error(
      `VITE_API_BASE_URL must be absolute or root-relative, got: ${apiBaseUrl}`,
    );
  }
}

const apiKey = import.meta.env.VITE_API_KEY as string | undefined;

export class ApiError extends Error {
  public status: number;
  public code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

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
    let errorMessage = `Request failed (${response.status}) for ${path}`;
    let errorCode = "unknown_error";

    try {
      const errorBody = await response.json();
      if (
        errorBody &&
        typeof errorBody === "object" &&
        "error" in errorBody &&
        errorBody.error &&
        typeof errorBody.error === "object"
      ) {
        const { code, message } = errorBody.error;
        if (typeof code === "string") errorCode = code;
        if (typeof message === "string") errorMessage = message;
      }
    } catch {
      // Response was not JSON or did not match expected error format
    }

    throw new ApiError(response.status, errorCode, errorMessage);
  }

  const body = (await response.json()) as ApiResponse<T>;
  return body.data;
}

export async function apiMutate<TBody, TResponse>(
  method: "POST" | "PUT" | "PATCH" | "DELETE",
  path: string,
  body?: TBody,
): Promise<TResponse> {
  const url = `${apiBaseUrl}${path}`;

  const headers = new Headers({
    Accept: "application/json",
    "Content-Type": "application/json",
  });

  if (apiKey) {
    headers.set("Authorization", `Bearer ${apiKey}`);
  }

  const response = await fetch(url, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    let errorMessage = `Request failed (${response.status}) for ${path}`;
    let errorCode = "unknown_error";

    try {
      const errorBody = await response.json();
      if (
        errorBody &&
        typeof errorBody === "object" &&
        "error" in errorBody &&
        errorBody.error &&
        typeof errorBody.error === "object"
      ) {
        const { code, message } = errorBody.error as {
          code?: string;
          message?: string;
        };
        if (typeof code === "string") errorCode = code;
        if (typeof message === "string") errorMessage = message;
      }
    } catch {
      // Response was not JSON
    }

    throw new ApiError(response.status, errorCode, errorMessage);
  }

  if (response.status === 204) return undefined as TResponse;
  const json = (await response.json()) as ApiResponse<TResponse>;
  return json.data;
}
