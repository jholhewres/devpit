import { useState } from 'react'

import type { PluginManifest } from '../gen/bindings'
import { CapabilityCard } from './CapabilityCard'
import { InstallCapability, uninstalledInWords, UninstallCapability } from './CapabilityDialogs'
import { addsInWords, CAPABILITIES_ICON, opensFor, summary } from './capabilities'
import { ask, commands } from './live'
import { usePlugins } from './usePlugins'
import { useShell } from './useShell'

/*
 * The capabilities devpit ships, and which this project installed and has on.
 *
 * Nothing to browse: every capability is compiled into the app. Installing
 * one is a choice per project, and both directions ask first.
 */

function Quiet({ title, detail }: { title: string; detail?: string | null }): React.JSX.Element {
  return (
    <div className="exempty capset__quiet">
      {CAPABILITIES_ICON(22)}
      <span className="exempty__t">{title}</span>
      {detail && <span className="exempty__d">{detail}</span>}
    </div>
  )
}

interface Uninstalling {
  readonly manifest: PluginManifest
  /** Null until its folder is read, and when it cannot be. */
  readonly files: number | null
}

export function PluginsPane(): React.JSX.Element {
  const { project, close, show } = useShell()
  const { plugins, error, setEnabled, install, uninstall } = usePlugins()
  const [installing, setInstalling] = useState<PluginManifest | null>(null)
  const [uninstalling, setUninstalling] = useState<Uninstalling | null>(null)
  /* Kept with its project, so a switch does not report another project's uninstall. */
  const [done, setDone] = useState<{ projectId: string; words: string } | null>(null)
  const said = summary(plugins)

  const askUninstall = (manifest: PluginManifest): void => {
    if (!project) return
    setUninstalling({ manifest, files: null })
    void ask(() => commands.pluginDataList(project.id, manifest.id)).then((answer) => {
      const files = answer.data?.files.length
      if (files !== undefined) setUninstalling((was) => (was?.manifest.id === manifest.id ? { ...was, files } : was))
    })
  }

  return (
    <>
      <div className="pane__bar">
        <span className="pane__ico">{CAPABILITIES_ICON(12)}</span>
        <span className="pane__t">
          <b>Capabilities</b>
          {said ? ` · ${said}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={() => close('plugins')} aria-label="Close Capabilities">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="list">
        <div className="list__in capset">
          <p className="list__note">
            Built into devpit, installed per project. On, a capability gets its own place in the
            sidebar; off, it leaves and its files stay.
          </p>

          {!project ? (
            <Quiet title="No project open." detail="Capabilities are chosen per project." />
          ) : plugins === null && error === null ? (
            /* The shape of a card while the catalogue is read, so the grid does not jump when it lands. */
            <div className="capgrid" aria-busy="true" aria-label="Reading capabilities">
              <div className="capcard capcard--ghost">
                <span className="capcard__ghost"></span>
                <span className="capcard__ghost"></span>
                <span className="capcard__ghost"></span>
              </div>
            </div>
          ) : plugins === null ? (
            <Quiet title="Capabilities could not be read." detail={error} />
          ) : plugins.length === 0 ? (
            <Quiet title="This build has no capabilities." />
          ) : (
            <>
              {error && (
                <p className="capset__problem" role="alert">
                  {error}
                </p>
              )}
              {done?.projectId === project.id && (
                <p className="capset__said" role="status">
                  {done.words}
                </p>
              )}
              <div className="capgrid">
                {plugins.map((plugin) => {
                  const opens = opensFor(plugin.manifest.id)
                  return (
                    <CapabilityCard
                      key={plugin.manifest.id}
                      plugin={plugin}
                      onSwitch={() => setEnabled(plugin.manifest.id, !plugin.enabled)}
                      onOpen={opens ? () => show(opens.kind, opens.tab) : null}
                      onInstall={() => setInstalling(plugin.manifest)}
                      onUninstall={() => askUninstall(plugin.manifest)}
                    />
                  )
                })}
              </div>
            </>
          )}
        </div>
      </div>

      {installing && (
        <InstallCapability
          name={installing.name}
          adds={addsInWords(installing)}
          onClose={() => setInstalling(null)}
          onConfirm={() => {
            install(installing.id)
            setDone(null)
            setInstalling(null)
          }}
        />
      )}
      {uninstalling && project && (
        <UninstallCapability
          name={uninstalling.manifest.name}
          files={uninstalling.files}
          onClose={() => setUninstalling(null)}
          onConfirm={(deleteData) => {
            const { manifest } = uninstalling
            const projectId = project.id
            setUninstalling(null)
            void uninstall(manifest.id, deleteData).then((removed) => {
              if (removed !== null) setDone({ projectId, words: uninstalledInWords(manifest.name, deleteData, removed) })
            })
          }}
        />
      )}
    </>
  )
}
