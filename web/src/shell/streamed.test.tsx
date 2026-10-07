import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

const opened = vi.fn()
let refusal: string | null = null

vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (token: string, scheme: string) => `${scheme}://localhost/${token}` }))
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => (refusal ? { data: null, error: refusal, loading: false } : { data: await call(), error: null, loading: false }),
  commands: { mediaOpen: (...args: unknown[]) => (opened(...args), 'tok123') },
}))

const { Streamed } = await import('./Streamed')

afterEach(() => {
  cleanup()
  refusal = null
  opened.mockClear()
})

describe('media streamed into a tab', () => {
  it("plays a project's video from devpit's scheme, asked for by its project and path", async () => {
    const { container } = render(<Streamed kind="video" projectId="p1" path="docs/demo.mp4" name="demo.mp4" size={1000} />)
    await screen.findByLabelText('demo.mp4')
    expect(container.querySelector('video')?.getAttribute('src')).toBe('devpitmedia://localhost/tok123')
    expect(opened).toHaveBeenCalledWith('p1', 'docs/demo.mp4')
  })

  it('asks for an absolute path without a project, and plays audio', async () => {
    const { container } = render(<Streamed kind="audio" projectId="p1" path="/home/me/.devpit/projects/p/pasted/voice.ogg" name="voice.ogg" size={10} />)
    await screen.findByLabelText('voice.ogg')
    expect(container.querySelector('audio')).toBeTruthy()
    expect(opened).toHaveBeenCalledWith(null, '/home/me/.devpit/projects/p/pasted/voice.ogg')
  })

  it('says why when the file is refused, or the window cannot play it', async () => {
    refusal = 'that path is not in a project'
    render(<Streamed kind="video" projectId="p1" path="/etc/x.mp4" name="x.mp4" size={1} />)
    expect(await screen.findByText('that path is not in a project')).toBeTruthy()
    cleanup()
    refusal = null
    render(<Streamed kind="video" projectId="p1" path="a.mov" name="a.mov" size={1} />)
    fireEvent.error(await screen.findByLabelText('a.mov'))
    expect(screen.getByText('This window could not play a.mov.')).toBeTruthy()
  })
})
