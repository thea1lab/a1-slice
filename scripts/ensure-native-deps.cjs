const { existsSync } = require('fs')
const { spawnSync } = require('child_process')
const { dirname, join } = require('path')

function pkgDir(name) {
  try {
    return dirname(require.resolve(`${name}/package.json`))
  } catch {
    return null
  }
}

function ensure(name, marker, script = 'install.js') {
  const dir = pkgDir(name)
  if (!dir) return
  if (marker && existsSync(join(dir, marker))) return
  const install = join(dir, script)
  if (!existsSync(install)) return
  const result = spawnSync(process.execPath, [install], { stdio: 'inherit', cwd: dir })
  if (result.status) process.exit(result.status ?? 1)
}

const electronBin = process.platform === 'win32' ? 'dist/electron.exe' : 'dist/electron'
ensure('electron', electronBin)
ensure('ffmpeg-static', process.platform === 'win32' ? 'ffmpeg.exe' : 'ffmpeg')
ensure('esbuild', process.platform === 'win32' ? 'esbuild.exe' : 'bin/esbuild')
