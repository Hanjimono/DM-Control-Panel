import Icon from "@/ui/Presentation/Icon"
import { cn } from "@/ui/Utils/cn"

import type { AppPage, SidebarProps } from "./types"

const NAV_ITEMS: { page: AppPage; icon: string; label: string }[] = [
  { page: "library", icon: "library_music", label: "Library" },
  { page: "settings", icon: "settings", label: "Settings" }
]

/**
 * Vertical navigation rail.
 * Matches the menu frame: 74px accent bar, mark on top, icon buttons below.
 *
 * @param page - Currently open page
 * @param onPageChange - Opens the given page
 */
export default function Sidebar({ page, onPageChange }: SidebarProps) {
  return (
    <aside className="sticky top-0 flex h-screen w-menu shrink-0 justify-center bg-accent px-4.25 py-10">
      <div className="flex w-full flex-col items-center gap-10">
        <div className="flex h-10 w-full items-center justify-center text-icon" aria-hidden="true">
          <Icon type="md" name="coffee" size={32} />
        </div>
        <nav className="flex w-full flex-col items-center gap-panel" aria-label="Main">
          {NAV_ITEMS.map((item) => {
            const isActive = item.page === page
            let ariaCurrent: "page" | undefined
            if (isActive) {
              ariaCurrent = "page"
            }

            return (
              <button
                key={item.page}
                type="button"
                aria-label={item.label}
                aria-current={ariaCurrent}
                className={cn(
                  "inline-flex size-6.75 cursor-pointer items-center justify-center border-none bg-transparent p-0 text-icon transition-colors duration-200",
                  "hover:text-foreground-hover",
                  "focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-action-primary",
                  isActive && "text-action-primary hover:text-action-primary"
                )}
                onClick={() => onPageChange(item.page)}
              >
                <Icon type="md" name={item.icon} size={24} />
              </button>
            )
          })}
        </nav>
      </div>
    </aside>
  )
}
