export function fileNameOf(path: string | null | undefined): string {
  if (!path) return ''
  const name = path.split(/[\\/]/).pop()
  return name || path
}
