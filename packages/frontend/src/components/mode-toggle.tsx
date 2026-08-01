import { Moon, Sun } from 'lucide-react'
import { useTheme } from './theme-provider'
import { Button } from './ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu'

export function ModeToggle() {
  const { setTheme } = useTheme()

  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant={'ghost'} size={'icon'} />}>
        <Sun className="h-[1.2rem] w-[1.2rem] scale-100 rotate-0 transition-all motion-reduce:transition-none dark:scale-0 dark:-rotate-90" />
        <Moon className="absolute h-[1.2rem] w-[1.2rem] scale-0 rotate-90 transition-all motion-reduce:transition-none dark:scale-100 dark:rotate-0" />
        <span className="sr-only">切换主题</span>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={() => setTheme('light')}>
          浅色
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => setTheme('dark')}>
          深色
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => setTheme('system')}>
          跟随系统
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
