/**
 * API module - Re-exports from modular structure.
 *
 * This file maintains backward compatibility with existing imports.
 * For new code, consider importing from specific modules:
 *
 * ```ts
 * // Import specific domain APIs
 * import { appsApi } from "@/lib/api/apps";
 * import { databasesApi } from "@/lib/api/databases";
 *
 * // Or use the combined api object
 * import { api } from "@/lib/api";
 * ```
 *
 * The API is now organized into domain-specific modules:
 * - core.ts      - Shared utilities (apiRequest, getStoredToken)
 * - projects.ts  - Project CRUD operations
 * - apps.ts      - App management, deployments, env vars, volumes
 * - databases.ts - Managed database operations and backups
 * - services.ts  - Docker Compose services and templates
 * - teams.ts     - Team management and membership
 * - notifications.ts - Notification channels and subscriptions
 * - git.ts       - Git providers, SSH keys, GitHub Apps
 * - system.ts    - System stats, health, audit logs
 * - previews.ts  - Preview deployments
 */

// Re-export everything from the modular structure
export * from "./api/index";

// Default export for backward compatibility
export { default } from "./api/index";
