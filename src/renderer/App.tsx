import { useCallback, useEffect, useReducer, useRef } from 'react'
import TitleBar from './TitleBar'
import ToolHeader from './components/ToolHeader'
import HomeScreen from './screens/HomeScreen'
import ToolStart from './components/ToolStart'
import TranscribeScreen from './screens/TranscribeScreen'
import TranscribeDoneScreen from './screens/TranscribeDoneScreen'
import FindScreen from './screens/FindScreen'
import ReviewScreen from './screens/ReviewScreen'
import ExportScreen from './screens/ExportScreen'
import ReframeScreen from './screens/ReframeScreen'
import CaptionsScreen from './screens/CaptionsScreen'
import FixWordsScreen from './screens/FixWordsScreen'
import { initialWizardState, wizardReducer, type WizardState } from './state'
import { useProject } from './hooks/useProject'
import { fileNameOf } from './lib/fileName'
import { isPlayerScreen, screenTitle } from './lib/screenTitle'
import type {
  AppSettings,
  CaptionLook,
  CaptionProject,
  CaptionStyle,
  ClipCrop,
  ClipSegment,
  ProgressUpdate,
  ToolId,
  TranscriptSegment
} from '../shared/types'
import { DEFAULT_CAPTION_STYLE } from '../shared/captions'
import { transcriptTextPath } from '../shared/project'
import { refineClipBounds } from '../shared/clipBounds'
import { withClipStatus } from '../shared/project'

function settingsOf(state: WizardState): AppSettings {
  return {
    provider: state.provider,
    model: state.model,
    apiKey: state.apiKey,
    apiKeys: state.apiKeys,
    userHint: state.userHint,
    language: state.language,
    entropyThold: state.entropyThold,
    maxContext: state.maxContext,
    beamSize: state.beamSize,
    temperatureInc: state.temperatureInc
  }
}

export default function App(): React.JSX.Element {
  const [state, dispatch] = useReducer(wizardReducer, initialWizardState)
  const { openTool, goHome } = useProject(dispatch)
  const clipDirty = useRef(false)

  useEffect(() => {
    if (state.videoPath) void window.api.allowVideoPath(state.videoPath)
  }, [state.videoPath])

  useEffect(() => {
    void window.api.loadSettings().then((saved) => {
      dispatch({
        type: 'LOAD_SETTINGS',
        provider: saved.provider,
        model: saved.model,
        apiKey: saved.apiKey,
        apiKeys: saved.apiKeys,
        userHint: saved.userHint,
        language: saved.language,
        entropyThold: saved.entropyThold ?? 2.8,
        maxContext: saved.maxContext ?? 64,
        beamSize: saved.beamSize ?? 5,
        temperatureInc: saved.temperatureInc ?? 0.1
      })
    })
  }, [])

  useEffect(() => {
    if (!state.settingsLoaded) return
    void window.api.saveSettings(settingsOf(state))
  }, [
    state.settingsLoaded,
    state.provider,
    state.model,
    state.apiKey,
    state.apiKeys,
    state.userHint,
    state.language,
    state.entropyThold,
    state.maxContext,
    state.beamSize,
    state.temperatureInc
  ])

  useEffect(() => {
    const unsubscribe = window.api.onProgress((update: ProgressUpdate) => {
      if (state.screen === 'transcribe') {
        dispatch({ type: 'TRANSCRIBE_PROGRESS', update })
      } else if (state.screen === 'find' && state.analyzing) {
        dispatch({ type: 'ANALYZE_PROGRESS', update })
      } else if (
        state.screen === 'export' ||
        state.screen === 'reframe' ||
        state.screen === 'captions'
      ) {
        dispatch({ type: 'EXPORT_PROGRESS', update })
      }
    })
    return unsubscribe
  }, [state.screen, state.analyzing])

  useEffect(() => {
    if (!clipDirty.current || !state.videoPath || !state.projectReady) return
    const path = state.videoPath
    const clips = state.clips
    const raw = state.rawResponse
    const timer = window.setTimeout(() => {
      clipDirty.current = false
      void window.api.saveClips(path, clips, raw)
    }, 300)
    return () => window.clearTimeout(timer)
  }, [state.clips, state.videoPath, state.projectReady, state.rawResponse])

  const markClips = (): void => {
    clipDirty.current = true
  }

  const enterTool = useCallback((tool: ToolId) => {
    dispatch({ type: 'ENTER_TOOL', tool })
  }, [])

  const chooseVideo = useCallback(
    async (tool: ToolId) => {
      const path = await window.api.selectVideo()
      if (path) await openTool(tool, path)
    },
    [openTool]
  )

  const runTranscribe = useCallback(async () => {
    if (!state.videoPath) return
    dispatch({ type: 'START_TRANSCRIBE' })
    const result = await window.api.transcribeVideo(
      state.videoPath,
      state.language,
      state.entropyThold,
      state.maxContext,
      state.beamSize,
      state.temperatureInc
    )
    if (result.success && result.segments) {
      dispatch({ type: 'TRANSCRIBE_DONE', segments: result.segments })
    } else {
      dispatch({
        type: 'TRANSCRIBE_ERROR',
        error: result.error || 'Transcription failed'
      })
    }
  }, [
    state.videoPath,
    state.language,
    state.entropyThold,
    state.maxContext,
    state.beamSize,
    state.temperatureInc
  ])

  const showTranscribe = (returnTo: 'find' | 'captions'): void => {
    dispatch({ type: 'PREPARE_TRANSCRIBE', returnTo })
  }

  const adoptSaved = useCallback((clips: ClipSegment[], rawResponse: string) => {
    const withStatus = withClipStatus(clips)
    dispatch({ type: 'ANALYZE_DONE', clips: withStatus, rawResponse })
    if (state.videoPath) void window.api.saveClips(state.videoPath, withStatus, rawResponse)
  }, [state.videoPath])

  const adoptClips = useCallback(
    (clips: ClipSegment[], rawResponse: string) => {
      const maxMs =
        state.segments.length > 0
          ? state.segments[state.segments.length - 1].endMs
          : state.videoDurationMs || Infinity
      const prepared = clips
        .map((clip) => {
          const refined = refineClipBounds(clip, state.segments)
          return {
            ...clip,
            ...refined,
            startMs: Math.max(0, Math.min(refined.startMs, maxMs)),
            endMs: Math.max(0, Math.min(refined.endMs, maxMs))
          }
        })
        .filter((clip) => clip.endMs > clip.startMs)
      if (prepared.length === 0) {
        dispatch({
          type: 'ANALYZE_ERROR',
          error: 'No clips found. Try different suggestions or a different video.'
        })
        return
      }
      const withStatus = withClipStatus(prepared)
      dispatch({ type: 'ANALYZE_DONE', clips: withStatus, rawResponse })
      if (state.videoPath) void window.api.saveClips(state.videoPath, withStatus, rawResponse)
    },
    [state.segments, state.videoDurationMs, state.videoPath]
  )

  const handleAnalyze = useCallback(
    async (agentId: string, userHint?: string) => {
      if (!state.videoPath) return
      dispatch({ type: 'START_ANALYZE' })
      const result = await window.api.analyzeTranscript(
        state.videoPath,
        state.segments,
        agentId,
        userHint
      )
      if (result.success && result.clips) {
        adoptClips(result.clips, result.rawResponse || '')
      } else {
        const stopped = result.error === 'Stopped.' || result.error === 'Cancelled'
        dispatch({ type: 'ANALYZE_ERROR', error: stopped ? '' : result.error || 'Analysis failed' })
      }
    },
    [state, adoptClips]
  )

  const handleSlice = useCallback(async () => {
    if (!state.videoPath) return
    const approved = state.clips.filter((clip) => clip.approved)
    const style = state.captions?.style ?? DEFAULT_CAPTION_STYLE
    dispatch({ type: 'START_EXPORT' })
    const result = await window.api.cutClips(
      state.videoPath,
      approved.map(({ title, startMs, endMs, category, topic }) => ({
        title,
        startMs,
        endMs,
        category,
        topic
      })),
      state.segments,
      'srt',
      style
    )
    if (result.success && result.outputDir) {
      dispatch({ type: 'EXPORT_DONE', outputDir: result.outputDir })
    } else {
      dispatch({ type: 'EXPORT_ERROR', error: result.error || 'Export failed' })
    }
  }, [state.videoPath, state.clips, state.segments, state.captions])

  const saveFraming = useCallback(
    (crop: ClipCrop) => {
      if (!state.videoPath || crop.ratio === 'original') return
      const framing = { ...state.framing, [crop.ratio]: crop }
      dispatch({ type: 'SET_FRAMING', framing })
      void window.api.saveFraming(state.videoPath, framing)
    },
    [state.videoPath, state.framing]
  )

  const handleCaptions = useCallback(
    (captions: CaptionProject) => {
      dispatch({ type: 'SET_CAPTIONS', captions })
      if (state.videoPath) void window.api.saveCaptions(state.videoPath, captions)
    },
    [state.videoPath]
  )

  const handleExportFrame = useCallback(
    async (crop: ClipCrop) => {
      if (!state.videoPath) return
      if (crop.ratio !== 'original') saveFraming(crop)
      dispatch({ type: 'START_RENDER' })
      const result = await window.api.exportReframed(state.videoPath, crop)
      if (result.success && result.outputDir) {
        dispatch({ type: 'EXPORT_DONE', outputDir: result.outputDir })
      } else {
        dispatch({ type: 'EXPORT_ERROR', error: result.error || 'Reframe failed' })
      }
    },
    [state.videoPath, saveFraming]
  )

  const saveFixedWords = useCallback(
    async (lines: TranscriptSegment[]): Promise<string | null> => {
      if (!state.videoPath) return 'Choose a video first.'
      const result = await window.api.saveTranscript(state.videoPath, lines)
      if (!result.success) return result.error || 'Could not save the transcript.'
      dispatch({ type: 'SET_SEGMENTS', segments: lines })
      if (state.captions) {
        const captions = { ...state.captions, source: 'transcript' as const, cues: lines }
        delete captions.filePath
        dispatch({ type: 'SET_CAPTIONS', captions })
        try {
          await window.api.saveCaptions(state.videoPath, captions)
        } catch {
          return 'The transcript was saved. The caption choices on this video could not be saved.'
        }
      }
      dispatch({ type: 'SHOW_SCREEN', screen: 'captions' })
      return null
    },
    [state.videoPath, state.captions]
  )

  const handleExportCaptions = useCallback(
    async (cues: TranscriptSegment[], look: CaptionLook, style: CaptionStyle) => {
      if (!state.videoPath) return
      dispatch({ type: 'START_RENDER' })
      const result = await window.api.exportCaptions(state.videoPath, cues, look, style)
      if (result.success && result.outputDir) {
        dispatch({ type: 'EXPORT_DONE', outputDir: result.outputDir })
      } else {
        dispatch({ type: 'EXPORT_ERROR', error: result.error || 'Caption export failed' })
      }
    },
    [state.videoPath]
  )

  const continueNote =
    state.returnTo === 'find'
      ? 'When it finishes, you return to Find best parts.'
      : state.returnTo === 'captions'
        ? 'When it finishes, you return to Captions.'
        : null

  const awaitingTool = toolAwaitingVideo(state)
  const player = isPlayerScreen(state.screen)
  const wideWork = state.screen === 'fix-words'
  const useSheet = state.screen !== 'home' && !wideWork && (!player || awaitingTool !== null)

  return (
    <div
      className="relative flex flex-col h-screen bg-bg-base text-[#f4f1ea] overflow-hidden"
    >
      <TitleBar>
        {state.screen !== 'home' && (
          <ToolHeader
            title={screenTitle(state.screen)}
            fileName={fileNameOf(state.videoPath)}
            onBack={goHome}
          />
        )}
      </TitleBar>
      <div
        className={
          useSheet
            ? 'stage custom-scrollbar'
            : 'relative flex flex-col flex-1 min-h-0 overflow-hidden'
        }
      >
        {renderScreen(state, {
          enterTool,
          chooseVideo,
          goHome,
          openTool,
          runTranscribe,
          showTranscribe,
          adoptClips,
          handleAnalyze,
          adoptSaved,
          handleSlice,
          saveFraming,
          handleCaptions,
          handleExportFrame,
          handleExportCaptions,
          saveFixedWords,
          markClips,
          continueNote,
          dispatch
        })}
      </div>
      {(state.screen === 'home' || useSheet || state.screen === 'fix-words') && <div className="sunset" />}
    </div>
  )
}

interface ScreenHandlers {
  enterTool: (tool: ToolId) => void
  chooseVideo: (tool: ToolId) => Promise<void>
  goHome: () => void
  openTool: (tool: ToolId, videoPath: string) => Promise<void>
  runTranscribe: () => Promise<void>
  showTranscribe: (returnTo: 'find' | 'captions') => void
  adoptClips: (clips: ClipSegment[], rawResponse: string) => void
  adoptSaved: (clips: ClipSegment[], rawResponse: string) => void
  handleAnalyze: (agentId: string, userHint?: string) => Promise<void>
  handleSlice: () => Promise<void>
  saveFraming: (crop: ClipCrop) => void
  handleCaptions: (captions: CaptionProject) => void
  handleExportFrame: (crop: ClipCrop) => Promise<void>
  handleExportCaptions: (cues: TranscriptSegment[], look: CaptionLook, style: CaptionStyle) => Promise<void>
  saveFixedWords: (lines: TranscriptSegment[]) => Promise<string | null>
  markClips: () => void
  continueNote: string | null
  dispatch: React.Dispatch<import('./state').WizardAction>
}

function toolAwaitingVideo(state: WizardState): ToolId | null {
  if (state.videoPath) return null
  if (state.screen === 'transcribe' && (state.transcribeStage === 'idle' || state.transcribeStage === 'done')) {
    return 'transcribe'
  }
  if (state.screen === 'find') return 'find'
  if (state.screen === 'reframe') return 'reframe'
  if (state.screen === 'captions') return 'captions'
  return null
}

function renderScreen(state: WizardState, handlers: ScreenHandlers): React.JSX.Element {
  const path = state.videoPath
  const awaiting = toolAwaitingVideo(state)
  if (awaiting) {
    return (
      <ToolStart
        tool={awaiting}
        onChoose={() => void handlers.chooseVideo(awaiting)}
        onUseRecent={(videoPath) => void handlers.openTool(awaiting, videoPath)}
      />
    )
  }
  switch (state.screen) {
    case 'home':
      return <HomeScreen onOpen={handlers.enterTool} />
    case 'transcribe':
      return (
        <TranscribeScreen
          projectReady={state.projectReady}
          videoPath={path || ''}
          language={state.language}
          entropyThold={state.entropyThold}
          maxContext={state.maxContext}
          beamSize={state.beamSize}
          temperatureInc={state.temperatureInc}
          savedCount={state.segments.length}
          continueNote={handlers.continueNote}
          stage={state.transcribeStage}
          message={state.transcribeMessage}
          percent={state.transcribePercent}
          error={state.transcribeError}
          onLanguageChange={(language) => handlers.dispatch({ type: 'SET_LANGUAGE', language })}
          onEntropyTholdChange={(entropyThold) =>
            handlers.dispatch({ type: 'SET_ENTROPY_THOLD', entropyThold })
          }
          onMaxContextChange={(maxContext) => handlers.dispatch({ type: 'SET_MAX_CONTEXT', maxContext })}
          onBeamSizeChange={(beamSize) => handlers.dispatch({ type: 'SET_BEAM_SIZE', beamSize })}
          onTemperatureIncChange={(temperatureInc) =>
            handlers.dispatch({ type: 'SET_TEMPERATURE_INC', temperatureInc })
          }
          onStart={() => void handlers.runTranscribe()}
          onUseSaved={() => handlers.dispatch({ type: 'USE_SAVED_TRANSCRIPT' })}
          onCancel={() => window.api.cancelPipeline()}
        />
      )
    case 'transcribe-done':
      return (
        <TranscribeDoneScreen
          videoPath={path || ''}
          segments={state.segments}
          onOpen={async () => {
            if (!path) return { success: false, error: 'Missing video' }
            return window.api.openTranscript(path, state.segments)
          }}
          onHome={handlers.goHome}
        />
      )
    case 'find':
      return (
        <FindScreen
          projectReady={state.projectReady}
          segments={state.segments}
          videoPath={path || ''}
          userHint={state.userHint}
          analyzing={state.analyzing}
          analyzePercent={state.analyzePercent}
          analyzeMessage={state.analyzeMessage}
          error={state.analyzeError}
          hasClips={state.clips.length > 0}
          onUserHintChange={(userHint) => handlers.dispatch({ type: 'SET_USER_HINT', userHint })}
          onAnalyze={(hint) => void handlers.handleAnalyze(hint)}
          onLoadCachedAnalysis={handlers.adoptSaved}
          onCancel={() => window.api.cancelPipeline()}
          onAddRange={() => {
            handlers.markClips()
            handlers.dispatch({ type: 'ADD_CLIP' })
          }}
          onBackToClips={() => handlers.dispatch({ type: 'SHOW_SCREEN', screen: 'review' })}
          onTranscribe={() => handlers.showTranscribe('find')}
        />
      )
    case 'review':
      return (
        <ReviewScreen
          clips={state.clips}
          videoPath={path || ''}
          rawResponse={state.rawResponse}
          videoDurationMs={state.videoDurationMs || undefined}
          onToggle={(id) => {
            handlers.markClips()
            handlers.dispatch({ type: 'TOGGLE_CLIP', id })
          }}
          onUpdateClipTimes={(id, startMs, endMs) => {
            handlers.markClips()
            handlers.dispatch({ type: 'UPDATE_CLIP_TIMES', id, startMs, endMs })
          }}
          onSlice={() => void handlers.handleSlice()}
          onFindAgain={() => handlers.dispatch({ type: 'SHOW_SCREEN', screen: 'find' })}
          onAddRange={() => {
            handlers.markClips()
            handlers.dispatch({ type: 'ADD_CLIP' })
          }}
        />
      )
    case 'export':
      return (
        <ExportScreen
          stage={state.exportStage}
          message={state.exportMessage}
          percent={state.exportPercent}
          outputDir={state.outputDir}
          error={state.exportError}
          onOpenFolder={() => state.outputDir && void window.api.openFolder(state.outputDir)}
          onStartOver={handlers.goHome}
          onCancel={() => window.api.cancelPipeline()}
          onReframe={() => path && void handlers.openTool('reframe', path)}
          onCaptions={() => path && void handlers.openTool('captions', path)}
        />
      )
    case 'reframe':
      if (!state.projectReady || !path) {
        return <p className="m-auto text-base text-neutral-400">Opening the video…</p>
      }
      return (
        <ReframeScreen
          videoPath={path}
          durationMs={state.videoDurationMs}
          framing={state.framing}
          exportStage={state.exportStage}
          exportMessage={state.exportMessage}
          exportPercent={state.exportPercent}
          outputDir={state.outputDir}
          exportError={state.exportError}
          onFramingChange={handlers.saveFraming}
          onExport={(crop) => void handlers.handleExportFrame(crop)}
          onOpenFolder={() => state.outputDir && void window.api.openFolder(state.outputDir)}
        />
      )
    case 'captions':
      if (!state.projectReady || !path) {
        return <p className="m-auto text-base text-neutral-400">Opening the video…</p>
      }
      return (
        <CaptionsScreen
          videoPath={path}
          durationMs={state.videoDurationMs}
          segments={state.segments}
          captions={state.captions}
          exportStage={state.exportStage}
          exportMessage={state.exportMessage}
          exportPercent={state.exportPercent}
          outputDir={state.outputDir}
          exportError={state.exportError}
          onChange={handlers.handleCaptions}
          onExport={(cues, look, style) => void handlers.handleExportCaptions(cues, look, style)}
          onFixWords={() => handlers.dispatch({ type: 'SHOW_SCREEN', screen: 'fix-words' })}
        />
      )
    case 'fix-words': {
      if (!state.projectReady || !path) {
        return <p className="m-auto text-base text-neutral-400">Opening the video…</p>
      }
      const lines =
        state.captions?.source === 'manual' && state.captions.cues.length > 0
          ? state.captions.cues
          : state.segments
      return (
        <FixWordsScreen
          fileName={fileNameOf(
            state.captions?.source === 'manual' && state.captions.filePath
              ? state.captions.filePath
              : transcriptTextPath(path)
          )}
          lines={lines}
          onBack={() => handlers.dispatch({ type: 'SHOW_SCREEN', screen: 'captions' })}
          onSave={handlers.saveFixedWords}
        />
      )
    }
    default:
      return <HomeScreen onOpen={handlers.enterTool} />
  }
}

