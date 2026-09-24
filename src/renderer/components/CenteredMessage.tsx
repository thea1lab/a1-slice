export default function CenteredMessage({
  children
}: {
  children: React.ReactNode
}): React.JSX.Element {
  return (
    <div className="flex flex-1 items-center justify-center">
      <p className="text-base text-neutral-400">{children}</p>
    </div>
  )
}
