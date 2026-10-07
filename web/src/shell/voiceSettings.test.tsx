import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Transcribing } from '../gen/bindings'

const downloaded = vi.fn()
const tested = vi.fn()
let format: string | null = 'audio/webm'
const settings: Transcribing = { engine: 'local', model: '', language: '', url: '', keySet: false, local: 'whisper-cli' } as Transcribing

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    transcribeRead: () => settings,
    transcribeModelDownload: () => (downloaded(), { ...settings, model: '/home/me/.devpit/models/ggml-base.bin' }),
    transcribeTest: (...args: unknown[]) => (tested(...args), { text: 'testing one two', note: null }),
  },
}))
vi.mock('./recorder', () => ({
  formatHere: () => format,
  record: async () => ({ stop: async () => new Blob(['x'], { type: 'audio/webm' }), cancel: () => {} }),
  refusal: () => 'The microphone is blocked.',
  systemLanguage: () => 'pt',
  base64Of: async () => 'eA==',
}))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: async () => null }))

const { VoiceSettings } = await import('./VoiceSettings')

afterEach(() => {
  cleanup()
  format = 'audio/webm'
})

describe('voice in Settings', () => {
  it("downloads whisper.cpp's base model when none is chosen, and uses it", async () => {
    render(<VoiceSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Download base model' }))
    expect(await screen.findByText('/home/me/.devpit/models/ggml-base.bin')).toBeTruthy()
    expect(downloaded).toHaveBeenCalledOnce()
    expect(screen.queryByRole('button', { name: 'Download base model' })).toBeNull()
  })

  it('tests the microphone and the engine at once, and says what it heard', async () => {
    render(<VoiceSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Test the microphone' }))
    expect(await screen.findByText('Heard: “testing one two”', {}, { timeout: 5000 })).toBeTruthy()
    expect(tested).toHaveBeenCalledWith('audio/webm', 'eA==', 'pt')
  })

  it('says why when this window cannot record', async () => {
    format = null
    render(<VoiceSettings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Test the microphone' }))
    expect(await screen.findByText(/GStreamer/)).toBeTruthy()
  })
})
