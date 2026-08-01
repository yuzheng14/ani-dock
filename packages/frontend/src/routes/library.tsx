import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/library')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/library"!</div>
}
