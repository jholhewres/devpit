import { describe, expect, it } from 'vitest'

import type { Project } from '../gen/bindings'
import { byAge } from './RailOrchestrators'

const made = (id: string): Project => ({ id }) as Project

describe('byAge', () => {
  it('keeps orchestrators where they were made, however the list arrives', () => {
    const lastOpenedFirst = [made('prj_01M3DT'), made('prj_01M3CT')]
    expect(byAge(lastOpenedFirst).map((one) => one.id)).toEqual(['prj_01M3CT', 'prj_01M3DT'])
    expect(byAge([...lastOpenedFirst].reverse()).map((one) => one.id)).toEqual(['prj_01M3CT', 'prj_01M3DT'])
  })
})
