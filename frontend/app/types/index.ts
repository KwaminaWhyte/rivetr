// Barrel re-export for all types.
// Prefer importing from "@/types/api" for domain-specific types,
// or "@/types" for shared generic types like PaginatedResponse.
export * from "./api";
export * from "./destinations";

// -------------------------------------------------------------------------
// Shared generic response types
// -------------------------------------------------------------------------

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}
