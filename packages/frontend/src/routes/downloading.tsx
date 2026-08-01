import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/downloading')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/downloading"!</div>
}
