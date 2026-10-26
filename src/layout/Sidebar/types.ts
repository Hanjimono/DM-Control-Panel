/** Pages opened from the sidebar. */
export type AppPage = "library" | "settings"

export interface SidebarProps {
  /** Currently open page */
  page: AppPage
  /** Opens the given page */
  onPageChange: (page: AppPage) => void
}
