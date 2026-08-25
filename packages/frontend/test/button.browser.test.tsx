import { expect, test, vi } from 'vitest'
import { render } from 'vitest-browser-react'

import { Button } from '@/components/ui/button'

test('renders and handles interaction in a browser', async () => {
  const onClick = vi.fn()
  const screen = await render(<Button onClick={onClick}>下载</Button>)
  const button = screen.getByRole('button', { name: '下载' })

  await expect.element(button).toBeVisible()
  await button.click()

  expect(onClick).toHaveBeenCalledOnce()
})
