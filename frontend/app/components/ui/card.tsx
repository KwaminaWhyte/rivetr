import * as React from "react"

import { cn } from "~/lib/utils"

/**
 * A Card that has a CardHeader as a direct child renders as a "section card":
 * a soft grey outer frame carrying a small underlined title, with the
 * CardContent as a white bordered inner body (same look as
 * components/page/SectionCard). Cards without a header stay plain white
 * tiles. Page classNames still win over both looks.
 */
const SectionedContext = React.createContext(false)

function hasHeaderChild(children: React.ReactNode): boolean {
  return React.Children.toArray(children).some(
    (child) => React.isValidElement(child) && child.type === CardHeader
  )
}

function Card({
  className,
  size = "default",
  children,
  ...props
}: React.ComponentProps<"div"> & { size?: "default" | "sm" }) {
  const sectioned = hasHeaderChild(children)
  return (
    <SectionedContext.Provider value={sectioned}>
      <div
        data-slot="card"
        data-size={size}
        data-sectioned={sectioned || undefined}
        className={cn(
          "text-card-foreground overflow-hidden text-sm group/card flex flex-col",
          sectioned
            ? "bg-frame gap-0 rounded-xl border p-1"
            : "ring-foreground/[0.08] shadow-xs bg-card gap-4 rounded-xl py-4 ring-1 has-data-[slot=card-footer]:pb-0 has-[>img:first-child]:pt-0 data-[size=sm]:gap-3 data-[size=sm]:py-3 data-[size=sm]:has-data-[slot=card-footer]:pb-0 *:[img:first-child]:rounded-t-xl *:[img:last-child]:rounded-b-xl",
          className
        )}
        {...props}
      >
        {children}
      </div>
    </SectionedContext.Provider>
  )
}

function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
  const sectioned = React.useContext(SectionedContext)
  return (
    <div
      data-slot="card-header"
      className={cn(
        "group/card-header @container/card-header grid auto-rows-min items-start has-data-[slot=card-action]:grid-cols-[1fr_auto] has-data-[slot=card-description]:grid-rows-[auto_auto]",
        sectioned
          ? "gap-0.5 px-3 pt-1.5 pb-2"
          : "gap-1 rounded-t-xl px-4 group-data-[size=sm]/card:px-3 [.border-b]:pb-4 group-data-[size=sm]/card:[.border-b]:pb-3",
        className
      )}
      {...props}
    />
  )
}

function CardTitle({ className, ...props }: React.ComponentProps<"div">) {
  const sectioned = React.useContext(SectionedContext)
  return (
    <div
      data-slot="card-title"
      className={cn(
        sectioned
          ? "text-sm leading-snug font-medium underline decoration-foreground/25 decoration-1 underline-offset-[5px] [&_svg:not([class*='size-'])]:size-4"
          : "text-base leading-snug font-medium group-data-[size=sm]/card:text-sm",
        className
      )}
      {...props}
    />
  )
}

function CardDescription({ className, ...props }: React.ComponentProps<"div">) {
  const sectioned = React.useContext(SectionedContext)
  return (
    <div
      data-slot="card-description"
      className={cn("text-muted-foreground", sectioned ? "mt-0.5 text-xs" : "text-sm", className)}
      {...props}
    />
  )
}

function CardAction({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-action"
      className={cn(
        "col-start-2 row-span-2 row-start-1 self-start justify-self-end",
        className
      )}
      {...props}
    />
  )
}

function CardContent({ className, ...props }: React.ComponentProps<"div">) {
  const sectioned = React.useContext(SectionedContext)
  return (
    <div
      data-slot="card-content"
      className={cn(
        sectioned
          ? "bg-card rounded-lg border p-4 shadow-xs sm:p-5"
          : "px-4 group-data-[size=sm]/card:px-3",
        className
      )}
      {...props}
    />
  )
}

function CardFooter({ className, ...props }: React.ComponentProps<"div">) {
  const sectioned = React.useContext(SectionedContext)
  return (
    <div
      data-slot="card-footer"
      className={cn(
        "flex items-center",
        sectioned
          ? "px-2 pt-2 pb-1"
          : "bg-muted/50 rounded-b-xl border-t p-4 group-data-[size=sm]/card:p-3",
        className
      )}
      {...props}
    />
  )
}

export {
  Card,
  CardHeader,
  CardFooter,
  CardTitle,
  CardAction,
  CardDescription,
  CardContent,
}
