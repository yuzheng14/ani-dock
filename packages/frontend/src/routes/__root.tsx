import { ModeToggle } from '@/components/mode-toggle'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
  useSidebar,
} from '@/components/ui/sidebar'
import { createRootRoute, Link, Outlet } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
import { HardDriveDownload, LibraryBig, Settings } from 'lucide-react'
import type { CSSProperties } from 'react'

function AppSidebar() {
  const { setOpenMobile } = useSidebar()
  return (
    <Sidebar variant="floating" collapsible="icon">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <div className="flex h-12 w-full items-center gap-2 overflow-hidden rounded-md p-2 text-sm group-data-[collapsible=icon]:size-8! group-data-[collapsible=icon]:p-0!">
              <div className="flex size-8 shrink-0 items-center justify-center">
                <img src="/logo.png" alt="" className="size-8" />
              </div>
              <span className="truncate text-lg font-medium">Ani Dock</span>
            </div>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                tooltip={'所有动画'}
                render={
                  <Link
                    to="/library"
                    activeProps={{ 'data-active': true }}
                    onClick={() => setOpenMobile(false)}
                  />
                }
              >
                <LibraryBig />
                <span>所有动画</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton
                tooltip={'下载列表'}
                render={
                  <Link
                    to="/downloading"
                    activeProps={{ 'data-active': true }}
                    onClick={() => setOpenMobile(false)}
                  />
                }
              >
                <HardDriveDownload />
                <span>下载列表</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              tooltip={'设置'}
              render={
                <Link
                  to="/settings"
                  activeProps={{ 'data-active': true }}
                  onClick={() => setOpenMobile(false)}
                />
              }
            >
              <Settings />
              <span>设置</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  )
}

const RootLayout = () => (
  <>
    <SidebarProvider style={{ '--sidebar-width': '18rem' } as CSSProperties}>
      <AppSidebar />
      <SidebarInset>
        <header className="flex h-16 shrink-0 items-center gap-2 p-3 transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-12">
          <SidebarTrigger />
          <div className="ml-auto">
            <ModeToggle />
          </div>
        </header>
        <Outlet />
      </SidebarInset>
    </SidebarProvider>
    <TanStackRouterDevtools />
    <ReactQueryDevtools />
  </>
)

export const Route = createRootRoute({ component: RootLayout })
