import { useEffect, useState } from "react";
import { Link, useLocation } from "react-router";
import { ChevronRight } from "lucide-react";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { useIsInstanceAdmin } from "@/lib/auth";
import {
  activeSidebarUrl,
  isSidebarItemActive,
  sidebarSections,
  type SidebarItem,
} from "@/lib/sidebar-nav";
import { cn } from "@/lib/utils";
import type { LucideIcon } from "lucide-react";

/** Small rounded-square bordered tile around a nav icon. */
export function NavIconTile({ icon: Icon, active }: { icon: LucideIcon; active?: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "inline-flex size-6 shrink-0 items-center justify-center rounded-md border bg-card shadow-xs transition-colors",
        active
          ? "border-primary/30 bg-primary/10 text-primary"
          : "text-muted-foreground group-hover/menu-item:text-foreground"
      )}
    >
      <Icon className="size-3.5" />
    </span>
  );
}

const BUTTON_CLASS =
  "h-9 gap-2.5 px-1.5 group-data-[collapsible=icon]:p-1! data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium";

function NavEntry({ item, activeUrl }: { item: SidebarItem; activeUrl: string | null }) {
  const { state, isMobile, setOpenMobile } = useSidebar();
  const isAdmin = useIsInstanceAdmin();
  const active = isSidebarItemActive(item, activeUrl);
  const children = item.children?.filter((c) => isAdmin || !c.adminOnly);
  const [open, setOpen] = useState(active);

  // Expand the submenu when navigation lands inside it.
  useEffect(() => {
    if (active) setOpen(true);
  }, [active]);

  const closeMobile = () => {
    if (isMobile) setOpenMobile(false);
  };

  if (!children || children.length === 0) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton asChild tooltip={item.title} isActive={active} className={BUTTON_CLASS}>
          <Link to={item.url} onClick={closeMobile}>
            <NavIconTile icon={item.icon} active={active} />
            <span>{item.title}</span>
          </Link>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  // Collapsed rail: the icon links straight to the first page of the group.
  if (state === "collapsed" && !isMobile) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton asChild tooltip={item.title} isActive={active} className={BUTTON_CLASS}>
          <Link to={item.url}>
            <NavIconTile icon={item.icon} active={active} />
            <span>{item.title}</span>
          </Link>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  return (
    <Collapsible asChild open={open} onOpenChange={setOpen}>
      <SidebarMenuItem>
        <CollapsibleTrigger asChild>
          <SidebarMenuButton tooltip={item.title} isActive={active && !open} className={BUTTON_CLASS}>
            <NavIconTile icon={item.icon} active={active} />
            <span>{item.title}</span>
            <ChevronRight
              className={cn(
                "ml-auto size-3.5! text-muted-foreground transition-transform duration-200",
                open && "rotate-90"
              )}
            />
          </SidebarMenuButton>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <SidebarMenuSub className="ml-[1.1rem] mr-0 mt-0.5">
            {children.map((child) => (
              <SidebarMenuSubItem key={child.url}>
                <SidebarMenuSubButton asChild isActive={activeUrl === child.url}>
                  <Link to={child.url} onClick={closeMobile}>
                    <span>{child.title}</span>
                  </Link>
                </SidebarMenuSubButton>
              </SidebarMenuSubItem>
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuItem>
    </Collapsible>
  );
}

/** Global navigation: top block plus "Infrastructure" and "Manage" sections. */
export function NavMain() {
  const { pathname } = useLocation();
  const activeUrl = activeSidebarUrl(pathname);

  return (
    <>
      {sidebarSections.map((section, i) => (
        <SidebarGroup key={section.label ?? i} className="py-1">
          {section.label && (
            <SidebarGroupLabel className="h-6 text-[11px] font-medium text-muted-foreground/80">
              {section.label}
            </SidebarGroupLabel>
          )}
          <SidebarMenu className="gap-0.5">
            {section.items.map((item) => (
              <NavEntry key={item.title} item={item} activeUrl={activeUrl} />
            ))}
          </SidebarMenu>
        </SidebarGroup>
      ))}
    </>
  );
}
