import { useState } from "react"

import Sidebar from "@/layout/Sidebar"
import type { AppPage } from "@/layout/Sidebar/types"
import LibraryPage from "@/pages/Library"
import SettingsPage from "@/pages/Settings"

/**
 * App shell: sidebar plus the open page.
 */
function App() {
  const [page, setPage] = useState<AppPage>("library")
  const showLibrary = page === "library"
  const showSettings = page === "settings"

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <Sidebar page={page} onPageChange={setPage} />
      <main className="min-w-0 flex-1 px-8 py-10">
        {showLibrary && <LibraryPage />}
        {showSettings && <SettingsPage />}
      </main>
    </div>
  )
}

export default App
