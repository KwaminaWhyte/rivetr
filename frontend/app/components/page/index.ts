/**
 * Shared page building blocks. Use these on every route so pages share one
 * look: PageHeader at the top, SectionCard for each block of content,
 * FormGrid + FormField for forms, DataTable for lists, EmptyState for empty
 * lists and the skeletons while loading.
 */
export { PageHeader, IconTile } from "./page-header";
export { SectionCard } from "./section-card";
export { FormGrid, FormField, SettingRow } from "./form-grid";
export { EmptyState } from "./empty-state";
export {
  DataTable,
  TableToolbar,
  TablePagination,
  usePagedRows,
  type DataColumn,
} from "./data-table";
export {
  ResourceStatus,
  StatusPill,
  StatusDot,
  statusTone,
  statusLabel,
  isActiveStatus,
  type Tone,
} from "./status-pill";
export { ViewToggle, useStoredPreference, type ViewOption } from "./view-toggle";
export { HeaderSkeleton, SectionSkeleton, CardGridSkeleton, ListSkeleton } from "./skeletons";
export { relativeTime, formatDuration } from "./format";
