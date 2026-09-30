import { describe, expect, it } from 'vitest'
import { agentLaunch, listInstalledAgents } from '../captionAgent'

describe('caption agents', () => {
  it('asks Grok 4.7 to print the transcript in one turn at low effort', () => {
    const launch = agentLaunch('grok', 'Edit this transcript.', '/tmp/job')
    expect(launch.command).toBe('grok')
    expect(launch.args).toContain('--no-plan')
    expect(launch.args).toContain('grok-4.7')
    expect(launch.args[launch.args.indexOf('--reasoning-effort') + 1]).toBe('low')
    expect(launch.args).not.toContain('plan')
    expect(launch.args).not.toContain('acceptEdits')
    expect(launch.args).not.toContain('read,edit')
    expect(launch.args[launch.args.indexOf('-p') + 1]).toBe('Edit this transcript.')
    expect(launch.stdin).toBeNull()
    expect(launch.streamJson).toBe(false)
  })

  it('gives Grok a prompt file when the transcript is very long', () => {
    const launch = agentLaunch('grok', 'x'.repeat(100_000), '/tmp/job')
    expect(launch.args.slice(0, 2)).toEqual(['--prompt-file', '/tmp/job/instructions.md'])
    expect(launch.args).toContain('grok-4.7')
    expect(launch.args[launch.args.indexOf('--reasoning-effort') + 1]).toBe('low')
    expect(launch.args).not.toContain('-p')
    expect(launch.stdin).toBeNull()
  })

  it('asks Sonnet 5.5 to print the edited transcript at low effort', () => {
    const launch = agentLaunch('claude', 'Edit this transcript.', '/tmp/job')
    expect(launch.command).toBe('claude')
    expect(launch.args).toContain('claude-sonnet-5-5')
    expect(launch.args[launch.args.indexOf('--effort') + 1]).toBe('low')
    expect(launch.args[launch.args.indexOf('--tools') + 1]).toBe('')
    expect(launch.args).not.toContain('plan')
    expect(launch.args.at(-1)).toBe('Edit this transcript.')
    expect(launch.stdin).toBeNull()
    expect(launch.streamJson).toBe(false)
  })

  it('asks Sol 5.6 to print the transcript without writing files', () => {
    const launch = agentLaunch('sol', 'Edit this transcript.', '/tmp/job')
    expect(launch.command).toBe('codex')
    expect(launch.args).toContain('gpt-5.6-sol')
    expect(launch.args).toContain('model_reasoning_effort="low"')
    expect(launch.args).toContain('read-only')
    expect(launch.args).toContain('/tmp/job')
    expect(launch.args.at(-1)).toBe('-')
    expect(launch.stdin).toBe('Edit this transcript.')
  })

  it('asks Gemini 3.8 Flash to print the transcript at low effort', () => {
    const launch = agentLaunch('agy', 'Edit this transcript.', '/tmp/job')
    expect(launch.command).toBe('agy')
    expect(launch.args).toContain('gemini-3.8-flash-low')
    expect(launch.args[launch.args.indexOf('--effort') + 1]).toBe('low')
    expect(launch.args[launch.args.indexOf('--print') + 1]).toBe('Edit this transcript.')
    expect(launch.args).not.toContain('plan')
    expect(launch.stdin).toBeNull()
  })

  it('lists only the agents installed on this computer', () => {
    expect(listInstalledAgents((command) => (command === 'grok' ? '/bin/grok' : null))).toEqual([
      { id: 'grok', label: 'Grok 4.7' }
    ])
    expect(
      listInstalledAgents((command) => (['grok', 'claude', 'codex', 'agy'].includes(command) ? `/bin/${command}` : null)).map(
        (agent) => agent.label
      )
    ).toEqual(['Grok 4.7', 'Sonnet 5.5', 'Sol 5.6', 'Gemini 3.8 Flash'])
  })
})
