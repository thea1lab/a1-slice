import { EventEmitter } from 'events'
import { describe, expect, it } from 'vitest'
import type { spawn } from 'child_process'
import { clipsFromAgentText, startClipFind } from '../clipAgent'
import type { TranscriptSegment } from '../../shared/types'

const lectureAnswer = `[{"title":"Classe é a resposta do modelo","start_id":24,"end_id":37,"category":"standalone"},{"title":"Taxa, lote e épocas","start_id":132,"end_id":149,"category":"related"},{"title":"Imagem nova vira floresta","start_id":161,"end_id":182,"category":"standalone"},{"title":"Elefante sem classe confunde o modelo","start_id":201,"end_id":211,"category":"standalone"},{"title":"Mudou o dado, treine de novo","start_id":227,"end_id":254,"category":"standalone"},{"title":"Sem exemplos o modelo erra","start_id":255,"end_id":267,"category":"standalone"}]`

const elephantAnswer = `[{"title":"O elefante confunde o modelo","start_id":195,"end_id":211,"category":"standalone"}]`

function lecture(): TranscriptSegment[] {
  return Array.from({ length: 280 }, (_, i) => ({
    startMs: i * 1000,
    endMs: (i + 1) * 1000,
    text: `line ${i}`
  }))
}

describe('clipsFromAgentText', () => {
  it('maps the lecture answer onto the numbered lines', () => {
    const segments = lecture()
    const clips = clipsFromAgentText(lectureAnswer, segments)
    expect(clips.map((clip) => clip.title)).toEqual([
      'Classe é a resposta do modelo',
      'Taxa, lote e épocas',
      'Imagem nova vira floresta',
      'Elefante sem classe confunde o modelo',
      'Mudou o dado, treine de novo',
      'Sem exemplos o modelo erra'
    ])
    expect(clips[0].startMs).toBe(segments[24].startMs)
    expect(clips[0].endMs).toBe(segments[37].endMs)
    expect(clips[3].category).toBe('standalone')
    expect(clips[1].category).toBe('related')
    expect(clips.every((clip, index) => index === 0 || clip.startMs >= clips[index - 1].endMs)).toBe(true)
  })

  it('keeps a single hinted clip', () => {
    const segments = lecture()
    const clips = clipsFromAgentText(`Note.\n${elephantAnswer}`, segments)
    expect(clips).toHaveLength(1)
    expect(clips[0].title).toBe('O elefante confunde o modelo')
    expect(clips[0].startMs).toBe(segments[195].startMs)
    expect(clips[0].endMs).toBe(segments[211].endMs)
  })

  it('returns nothing when the agent finds no clip', () => {
    expect(clipsFromAgentText('[]', lecture())).toEqual([])
  })
})

function fakeSpawn(stdout: string, closeOnStart = true): ReturnType<typeof spawn> {
  const child = new EventEmitter() as ReturnType<typeof spawn>
  const pipe = (): EventEmitter & { setEncoding: () => void } => {
    const stream = new EventEmitter() as EventEmitter & { setEncoding: () => void }
    stream.setEncoding = () => {}
    return stream
  }
  child.stdout = pipe() as unknown as ReturnType<typeof spawn>['stdout']
  child.stderr = pipe() as unknown as ReturnType<typeof spawn>['stderr']
  child.stdin = {
    write() {
      return true
    },
    end() {},
    on() {
      return child.stdin
    }
  } as unknown as ReturnType<typeof spawn>['stdin']
  child.kill = () => {
    child.emit('close', 0)
    return true
  }
  if (closeOnStart) {
    setImmediate(() => {
      child.stdout?.emit('data', stdout)
      child.emit('close', 0)
    })
  }
  return child
}

describe('startClipFind', () => {
  it('returns clips from the agent printout', async () => {
    const logs: string[] = []
    const run = startClipFind(
      'grok',
      lecture(),
      undefined,
      (line) => logs.push(line),
      (() => fakeSpawn(elephantAnswer)) as typeof spawn
    )
    const result = await run.done
    expect(result.clips).toHaveLength(1)
    expect(result.clips?.[0].title).toBe('O elefante confunde o modelo')
    expect(logs[0]).toBe('Starting Grok 4.7.')
    expect(logs).toContain('Found 1 clip.')
  })

  it('stops when the run is cancelled', async () => {
    const run = startClipFind(
      'grok',
      lecture(),
      undefined,
      () => {},
      (() => fakeSpawn('', false)) as typeof spawn
    )
    run.kill()
    await expect(run.done).resolves.toEqual({ error: 'Stopped.' })
  })
})
