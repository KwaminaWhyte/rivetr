import { useEffect, useMemo, useState, type ReactNode } from "react";
import { ChevronLeft, ChevronRight, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { usePersistedState } from "@/lib/persisted-state";

const isPageSize = (v: unknown): v is number => Number.isInteger(v) && (v as number) > 0 && (v as number) <= 500;

export interface DataColumn<T> {
  id: string;
  header: ReactNode;
  cell: (row: T) => ReactNode;
  className?: string;
  headerClassName?: string;
  /** Hide on narrow screens: "sm" | "md" | "lg" means visible from that breakpoint up. */
  showFrom?: "sm" | "md" | "lg";
}

const SHOW_FROM: Record<NonNullable<DataColumn<unknown>["showFrom"]>, string> = {
  sm: "hidden sm:table-cell",
  md: "hidden md:table-cell",
  lg: "hidden lg:table-cell",
};

interface DataTableProps<T> {
  columns: DataColumn<T>[];
  rows: T[];
  getRowKey: (row: T) => string;
  onRowClick?: (row: T) => void;
  loading?: boolean;
  /** Rendered in place of the body when there are no rows. */
  empty?: ReactNode;
  /** Optional footer, usually <TablePagination>. */
  footer?: ReactNode;
  /** Optional toolbar above the table, usually <TableToolbar>. */
  toolbar?: ReactNode;
  className?: string;
  rowClassName?: (row: T) => string | undefined;
}

/**
 * Shared table style: light header, roomy rows, hover highlight, optional
 * toolbar and pagination. Columns can hide on small screens via `showFrom`.
 */
export function DataTable<T>({
  columns,
  rows,
  getRowKey,
  onRowClick,
  loading,
  empty,
  footer,
  toolbar,
  className,
  rowClassName,
}: DataTableProps<T>) {
  return (
    <div className={cn("overflow-hidden rounded-lg border bg-card shadow-xs", className)}>
      {toolbar && <div className="border-b px-3 py-2.5">{toolbar}</div>}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted/40">
            <tr className="border-b">
              {columns.map((col) => (
                <th
                  key={col.id}
                  scope="col"
                  className={cn(
                    "h-[calc(var(--density-head-h)-0.25rem)] whitespace-nowrap px-4 text-left align-middle text-xs font-medium text-muted-foreground",
                    col.showFrom && SHOW_FROM[col.showFrom],
                    col.headerClassName
                  )}
                >
                  {col.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {loading ? (
              Array.from({ length: 4 }).map((_, i) => (
                <tr key={i} className="border-b last:border-0">
                  {columns.map((col) => (
                    <td
                      key={col.id}
                      className={cn("px-4 py-(--density-row-py)", col.showFrom && SHOW_FROM[col.showFrom])}
                    >
                      <Skeleton className="h-4 w-full max-w-40" />
                    </td>
                  ))}
                </tr>
              ))
            ) : rows.length === 0 ? (
              <tr>
                <td colSpan={columns.length} className="p-0">
                  {empty ?? (
                    <p className="px-4 py-10 text-center text-sm text-muted-foreground">
                      Nothing to show.
                    </p>
                  )}
                </td>
              </tr>
            ) : (
              rows.map((row) => (
                <tr
                  key={getRowKey(row)}
                  onClick={onRowClick ? () => onRowClick(row) : undefined}
                  className={cn(
                    "border-b transition-colors last:border-0 hover:bg-muted/40",
                    onRowClick && "cursor-pointer",
                    rowClassName?.(row)
                  )}
                >
                  {columns.map((col) => (
                    <td
                      key={col.id}
                      className={cn(
                        "px-4 py-(--density-row-py) align-middle",
                        col.showFrom && SHOW_FROM[col.showFrom],
                        col.className
                      )}
                    >
                      {col.cell(row)}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {footer && <div className="border-t px-3 py-2">{footer}</div>}
    </div>
  );
}

/** Search box on the left, filters and extra controls on the right. */
export function TableToolbar({
  search,
  onSearchChange,
  searchPlaceholder = "Search",
  children,
  className,
}: {
  search?: string;
  onSearchChange?: (value: string) => void;
  searchPlaceholder?: string;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-wrap items-center gap-2", className)}>
      {onSearchChange && (
        <div className="relative w-full min-w-0 sm:w-auto sm:flex-1 sm:max-w-xs">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            aria-label={searchPlaceholder}
            placeholder={searchPlaceholder}
            value={search ?? ""}
            onChange={(e) => onSearchChange(e.target.value)}
            className="pl-8"
          />
        </div>
      )}
      {children && <div className="flex flex-wrap items-center gap-2 sm:ml-auto">{children}</div>}
    </div>
  );
}

/** "1-10 of 17 · 10 / page" with previous and next buttons. */
export function TablePagination({
  page,
  pageSize,
  total,
  onPageChange,
  onPageSizeChange,
  pageSizes = [10, 20, 50],
  className,
}: {
  page: number;
  pageSize: number;
  total: number;
  onPageChange: (page: number) => void;
  onPageSizeChange?: (size: number) => void;
  pageSizes?: number[];
  className?: string;
}) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const from = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const to = Math.min(total, page * pageSize);
  return (
    <div className={cn("flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground", className)}>
      <span>
        {from}-{to} of {total}
      </span>
      <div className="flex items-center gap-2">
        {onPageSizeChange && (
          <Select value={String(pageSize)} onValueChange={(v) => onPageSizeChange(Number(v))}>
            <SelectTrigger size="sm" className="w-[6.5rem]" aria-label="Rows per page">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {pageSizes.map((s) => (
                <SelectItem key={s} value={String(s)}>
                  {s} / page
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
        <Button
          variant="outline"
          size="icon-sm"
          aria-label="Previous page"
          disabled={page <= 1}
          onClick={() => onPageChange(page - 1)}
        >
          <ChevronLeft />
        </Button>
        <span className="tabular-nums">
          {page} / {pages}
        </span>
        <Button
          variant="outline"
          size="icon-sm"
          aria-label="Next page"
          disabled={page >= pages}
          onClick={() => onPageChange(page + 1)}
        >
          <ChevronRight />
        </Button>
      </div>
    </div>
  );
}

/**
 * Client-side pagination for an in-memory list. Resets to page 1 when the list
 * shrinks. Pass `persistKey` to remember the page size per table
 * (rivetr:ui:table:<key>:page-size).
 */
export function usePagedRows<T>(rows: T[], initialPageSize = 10, persistKey?: string) {
  const [page, setPage] = useState(1);
  const [storedSize, setStoredSize] = usePersistedState<number>(
    `table:${persistKey ?? "_"}:page-size`,
    initialPageSize,
    { validate: isPageSize }
  );
  const [localSize, setLocalSize] = useState(initialPageSize);
  const pageSize = persistKey ? storedSize : localSize;
  const setPageSize = persistKey ? setStoredSize : setLocalSize;
  const pages = Math.max(1, Math.ceil(rows.length / pageSize));
  useEffect(() => {
    if (page > pages) setPage(1);
  }, [page, pages]);
  const pageRows = useMemo(
    () => rows.slice((page - 1) * pageSize, page * pageSize),
    [rows, page, pageSize]
  );
  return {
    page,
    pageSize,
    pageRows,
    total: rows.length,
    setPage,
    setPageSize: (size: number) => {
      setPageSize(size);
      setPage(1);
    },
  };
}
