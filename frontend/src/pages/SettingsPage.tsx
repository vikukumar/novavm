import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { settingsApi } from '@/lib/api'
import type { AppSettings, HypervisorInfo, Theme } from '@/types'
import { cn } from '@/lib/utils'
import { toast } from '@/components/ui/use-toast'

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [hypervisor, setHypervisor] = useState<HypervisorInfo | null>(null)
  const [version, setVersion] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    Promise.all([
      settingsApi.get(),
      settingsApi.getHypervisorInfo(),
      settingsApi.getVersion(),
    ]).then(([s, h, v]) => {
      setSettings(s)
      setHypervisor(h)
      setVersion(v)
    }).catch(() => {
      // Backend may not be running in dev — use defaults
      setSettings({
        theme: 'dark',
        default_storage_dir: './vms',
        default_iso_dir: './iso',
        auto_start_service: true,
        metrics_interval_secs: 1,
        telemetry_enabled: false,
        language: 'en-US',
      })
      setVersion('0.1.0-dev')
    })
  }, [])

  const save = async () => {
    if (!settings) return
    setSaving(true)
    try {
      await settingsApi.update(settings)
      toast({ title: 'Settings saved' })
    } catch (e) {
      toast({ title: 'Failed to save settings', description: String(e), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  if (!settings) {
    return <div className="space-y-4">{Array.from({ length: 5 }).map((_, i) => <div key={i} className="skeleton h-12 rounded-xl" />)}</div>
  }

  const THEMES: { label: string; value: Theme }[] = [
    { label: '🌙 Dark', value: 'dark' },
    { label: '☀️ Light', value: 'light' },
    { label: '💻 System', value: 'system' },
  ]

  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="max-w-2xl mx-auto space-y-8">
      <div className="flex items-center gap-4">
        <img
          src="/novavm_icon.png"
          alt="NovaVM Logo"
          className="w-12 h-12 rounded-xl shadow-lg border border-white/10 object-cover"
        />
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Settings</h2>
          <p className="text-muted-foreground text-sm mt-0.5">NovaVM v{version}</p>
        </div>
      </div>

      {/* Appearance */}
      <Section title="Appearance">
        <div className="space-y-1">
          <label className="text-sm font-medium">Theme</label>
          <div className="flex gap-2">
            {THEMES.map((t) => (
              <button
                key={t.value}
                onClick={() => setSettings({ ...settings, theme: t.value })}
                className={cn(
                  'px-3 py-1.5 text-sm rounded-lg border transition-colors',
                  settings.theme === t.value
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'bg-muted border-border hover:bg-accent',
                )}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>
      </Section>

      {/* Storage */}
      <Section title="Storage">
        <Field
          label="Default VM directory"
          value={settings.default_storage_dir}
          onChange={(v) => setSettings({ ...settings, default_storage_dir: v })}
        />
        <Field
          label="Default ISO directory"
          value={settings.default_iso_dir}
          onChange={(v) => setSettings({ ...settings, default_iso_dir: v })}
        />
      </Section>

      {/* Service */}
      <Section title="Background Service">
        <CheckField
          label="Start service automatically on login"
          checked={settings.auto_start_service}
          onChange={(v) => setSettings({ ...settings, auto_start_service: v })}
        />
        <CheckField
          label="Enable telemetry (crash reports, anonymous usage)"
          checked={settings.telemetry_enabled}
          onChange={(v) => setSettings({ ...settings, telemetry_enabled: v })}
        />
      </Section>

      {/* Hypervisor info */}
      {hypervisor && (
        <Section title="Hypervisor">
          <div className="grid grid-cols-2 gap-3 text-sm">
            <InfoItem label="Backend" value={hypervisor.backend_name} />
            <InfoItem label="Version" value={hypervisor.backend_version} />
            <InfoItem label="Secure Boot" value={hypervisor.secure_boot ? '✅' : '❌'} />
            <InfoItem label="vTPM" value={hypervisor.vtpm ? '✅' : '❌'} />
            <InfoItem label="Nested Virt" value={hypervisor.nested_virt ? '✅' : '❌'} />
            <InfoItem label="Huge Pages" value={hypervisor.huge_pages ? '✅' : '❌'} />
            <InfoItem label="Ballooning" value={hypervisor.memory_ballooning ? '✅' : '❌'} />
            <InfoItem label="Dedup (KSM)" value={hypervisor.memory_dedup ? '✅' : '❌'} />
          </div>
        </Section>
      )}

      {/* Save */}
      <button
        onClick={save}
        disabled={saving}
        className="w-full py-2.5 text-sm font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
      >
        {saving ? 'Saving…' : 'Save Settings'}
      </button>
    </motion.div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold text-sm text-muted-foreground uppercase tracking-wider">{title}</h3>
      {children}
    </div>
  )
}

function Field({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  return (
    <div className="space-y-1">
      <label className="text-sm font-medium">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full px-3 py-2 text-sm rounded-lg bg-muted border border-border focus:outline-none focus:ring-2 focus:ring-ring"
      />
    </div>
  )
}

function CheckField({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center gap-3 text-sm cursor-pointer">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="rounded"
      />
      {label}
    </label>
  )
}

function InfoItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground mb-0.5">{label}</p>
      <p className="font-medium text-sm">{value}</p>
    </div>
  )
}
