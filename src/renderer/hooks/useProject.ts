import { useCallback, useRef } from 'react'
import type { Dispatch } from 'react'
import type { ToolId } from '../../shared/types'
import { withClipStatus } from '../../shared/project'
import type { WizardAction } from '../state'

export function useProject(dispatch: Dispatch<WizardAction>): {
  openTool: (tool: ToolId, videoPath: string) => Promise<void>
  goHome: () => void
} {
  const generation = useRef(0)

  const openTool = useCallback(
    async (tool: ToolId, videoPath: string) => {
      const ticket = ++generation.current
      dispatch({ type: 'OPEN_TOOL', tool, videoPath })
      void window.api.rememberVideo(videoPath)
      const project = await window.api.loadProject(videoPath)
      if (ticket !== generation.current) return
      dispatch({
        type: 'PROJECT_LOADED',
        segments: project.segments,
        clips: withClipStatus(project.clips),
        rawResponse: project.rawResponse,
        framing: project.framing,
        captions: project.captions,
        durationMs: project.durationMs
      })
    },
    [dispatch]
  )

  const goHome = useCallback(() => {
    generation.current += 1
    dispatch({ type: 'GO_HOME' })
  }, [dispatch])

  return { openTool, goHome }
}
