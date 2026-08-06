import { useState, useEffect } from 'react'
import {
  Code, Users, Play, RefreshCw, UserPlus, Key, ShieldCheck,
  CheckCircle2, XCircle, Clock, Terminal, Lock,
} from 'lucide-react'
import { vmApi } from '@/lib/api'
import { toast } from '@/components/ui/use-toast'
import { cn } from '@/lib/utils'

interface VmScriptingAndUserTabProps {
  vmId: string
  vmName: string
  vmState: string
}

interface GuestUserItem {
  username: string
  full_name: string
  is_admin: boolean
  is_disabled: boolean
  last_login?: string
}

interface ScriptResult {
  exit_code: number
  stdout: string
  stderr: string
  duration_ms: number
}

const PRESET_SCRIPTS = [
  {
    name: 'System Information Audit',
    interpreter: 'powershell',
    code: `Get-ComputerInfo | Select-Object OsName, OsVersion, CsName, OsArchitecture, WindowsVersion`,
  },
  {
    name: 'Network Adapter Config',
    interpreter: 'powershell',
    code: `Get-NetIPAddress -AddressFamily IPv4 | Select-Object InterfaceAlias, IPAddress, PrefixLength`,
  },
  {
    name: 'Linux Disk & Memory Audit',
    interpreter: 'bash',
    code: `echo "=== DISK SPACE ===" && df -h && echo "" && echo "=== MEMORY ===" && free -m`,
  },
  {
    name: 'List Active Process Tree',
    interpreter: 'powershell',
    code: `Get-Process | Sort-Object CPU -Descending | Select-Object -First 10 Id, ProcessName, CPU, WorkingSet64`,
  },
]

export function VmScriptingAndUserTab({ vmId, vmName, vmState }: VmScriptingAndUserTabProps) {
  const [subTab, setSubTab] = useState<'scripting' | 'users'>('scripting')

  // ── Script Execution State ──
  const [interpreter, setInterpreter] = useState<string>('powershell')
  const [scriptBody, setScriptBody] = useState<string>(PRESET_SCRIPTS[0].code)
  const [runningScript, setRunningScript] = useState<boolean>(false)
  const [lastResult, setLastResult] = useState<ScriptResult | null>(null)

  // ── User Management State ──
  const [users, setUsers] = useState<GuestUserItem[]>([])
  const [loadingUsers, setLoadingUsers] = useState<boolean>(false)
  const [showAddUserModal, setShowAddUserModal] = useState<boolean>(false)
  const [newUsername, setNewUsername] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [newFullName, setNewFullName] = useState('')
  const [newIsAdmin, setNewIsAdmin] = useState(false)
  const [creatingUser, setCreatingUser] = useState(false)

  // ── Password Reset State ──
  const [resetTargetUser, setResetTargetUser] = useState<string | null>(null)
  const [resetNewPassword, setResetNewPassword] = useState('')
  const [updatingPassword, setUpdatingPassword] = useState(false)

  const isVmRunning = vmState === 'running'

  // Fetch users when tab changes to 'users'
  useEffect(() => {
    if (subTab === 'users') {
      fetchUsers()
    }
  }, [subTab, vmId])

  const fetchUsers = async () => {
    setLoadingUsers(true)
    try {
      const list = await vmApi.listGuestUsers(vmId)
      setUsers(list)
    } catch (e) {
      toast({ title: 'Could not list guest users', description: String(e), variant: 'destructive' })
    } finally {
      setLoadingUsers(false)
    }
  }

  const handleSyncUsers = async () => {
    setLoadingUsers(true)
    try {
      const synced = await vmApi.syncGuestUsers(vmId)
      setUsers(synced)
      toast({
        title: 'Users Synchronized',
        description: `Successfully synchronized ${synced.length} OS user accounts between NovaVM Portal and '${vmName}'.`,
      })
    } catch (e) {
      toast({ title: 'User Sync Failed', description: String(e), variant: 'destructive' })
    } finally {
      setLoadingUsers(false)
    }
  }

  const handleRunScript = async () => {
    if (!scriptBody.trim()) return
    setRunningScript(true)
    setLastResult(null)
    try {
      const res = await vmApi.runScript(vmId, scriptBody, interpreter)
      setLastResult(res)
      if (res.exit_code === 0) {
        toast({ title: 'Script Executed Successfully', description: `Completed in ${res.duration_ms} ms (exit code 0)` })
      } else {
        toast({ title: 'Script Execution Non-Zero Exit', description: `Exit code ${res.exit_code}`, variant: 'destructive' })
      }
    } catch (e) {
      toast({ title: 'Script Failed to Launch', description: String(e), variant: 'destructive' })
    } finally {
      setRunningScript(false)
    }
  }

  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newUsername || !newPassword) return
    setCreatingUser(true)
    try {
      const newUser = await vmApi.createGuestUser(vmId, {
        username: newUsername,
        password: newPassword,
        full_name: newFullName || newUsername,
        is_admin: newIsAdmin,
      })
      setUsers(prev => [...prev.filter(u => u.username !== newUser.username), newUser])
      toast({ title: 'Guest User Created', description: `User '${newUser.username}' created inside '${vmName}' OS and synchronized.` })
      setShowAddUserModal(false)
      setNewUsername('')
      setNewPassword('')
      setNewFullName('')
      setNewIsAdmin(false)
    } catch (err) {
      toast({ title: 'Failed to Create User', description: String(err), variant: 'destructive' })
    } finally {
      setCreatingUser(false)
    }
  }

  const handleResetPassword = async () => {
    if (!resetTargetUser || !resetNewPassword) return
    setUpdatingPassword(true)
    try {
      await vmApi.updateGuestUserPassword(vmId, resetTargetUser, resetNewPassword)
      toast({ title: 'Password Updated', description: `Updated credentials for guest OS user '${resetTargetUser}'.` })
      setResetTargetUser(null)
      setResetNewPassword('')
    } catch (e) {
      toast({ title: 'Password Update Failed', description: String(e), variant: 'destructive' })
    } finally {
      setUpdatingPassword(false)
    }
  }

  return (
    <div className="space-y-6">
      {/* Top Toggle Switch */}
      <div className="flex items-center justify-between bg-muted/40 p-1.5 rounded-xl border border-border">
        <div className="flex gap-2">
          <button
            onClick={() => setSubTab('scripting')}
            className={cn(
              'flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold transition-all',
              subTab === 'scripting' ? 'bg-primary text-primary-foreground shadow-md' : 'text-muted-foreground hover:text-foreground hover:bg-muted/60'
            )}
          >
            <Code size={14} />
            In-Guest Script Execution
          </button>
          <button
            onClick={() => setSubTab('users')}
            className={cn(
              'flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold transition-all',
              subTab === 'users' ? 'bg-primary text-primary-foreground shadow-md' : 'text-muted-foreground hover:text-foreground hover:bg-muted/60'
            )}
          >
            <Users size={14} />
            OS User Management & Sync
          </button>
        </div>

        <div className="flex items-center gap-2 text-[11px] text-muted-foreground pr-3">
          <ShieldCheck size={13} className="text-emerald-400" />
          <span>NovaVM Guest Tools Active</span>
        </div>
      </div>

      {/* ── SUB-TAB 1: SCRIPT EXECUTION ── */}
      {subTab === 'scripting' && (
        <div className="space-y-4">
          <div className="rounded-2xl border border-border bg-card p-5 space-y-4 shadow-xl">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
              <div>
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Terminal size={16} className="text-primary" />
                  VMware Tools Guest Execution Engine
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Execute custom code (Bash, PowerShell, Python, CMD) directly inside '{vmName}' OS.
                </p>
              </div>

              {/* Preset Selector */}
              <div className="flex items-center gap-2">
                <span className="text-xs text-muted-foreground font-medium">Presets:</span>
                <select
                  onChange={(e) => {
                    const preset = PRESET_SCRIPTS.find(p => p.name === e.target.value)
                    if (preset) {
                      setInterpreter(preset.interpreter)
                      setScriptBody(preset.code)
                    }
                  }}
                  className="bg-background border border-border text-xs rounded-lg px-2.5 py-1.5 text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  {PRESET_SCRIPTS.map(p => (
                    <option key={p.name} value={p.name}>{p.name}</option>
                  ))}
                </select>
              </div>
            </div>

            {/* Config & Controls */}
            <div className="grid grid-cols-1 sm:grid-cols-4 gap-3 items-center">
              <div>
                <label className="text-[11px] font-semibold text-muted-foreground block mb-1">Interpreter</label>
                <select
                  value={interpreter}
                  onChange={(e) => setInterpreter(e.target.value)}
                  className="w-full bg-background border border-border text-xs rounded-lg px-3 py-2 text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  <option value="powershell">PowerShell (Windows)</option>
                  <option value="cmd">Command Prompt (CMD)</option>
                  <option value="bash">Bash / Shell (Linux)</option>
                  <option value="python">Python 3</option>
                </select>
              </div>

              <div className="sm:col-span-3 flex justify-end items-end h-full">
                <button
                  onClick={handleRunScript}
                  disabled={runningScript || !isVmRunning}
                  className={cn(
                    'flex items-center justify-center gap-2 px-5 py-2 rounded-xl text-xs font-semibold transition-all shadow-lg',
                    isVmRunning
                      ? 'bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer'
                      : 'bg-muted text-muted-foreground cursor-not-allowed border border-border'
                  )}
                >
                  {runningScript ? (
                    <>
                      <div className="w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin" />
                      Executing in Guest OS...
                    </>
                  ) : (
                    <>
                      <Play size={14} />
                      Execute Code in VM
                    </>
                  )}
                </button>
              </div>
            </div>

            {/* Script Code Area */}
            <div>
              <textarea
                value={scriptBody}
                onChange={(e) => setScriptBody(e.target.value)}
                rows={6}
                placeholder="Type script code to execute inside guest OS..."
                className="w-full bg-[#080808] border border-border/80 rounded-xl p-4 font-mono text-xs text-emerald-300 focus:outline-none focus:ring-1 focus:ring-primary leading-relaxed shadow-inner"
              />
            </div>
          </div>

          {/* Execution Result Box */}
          {lastResult && (
            <div className="rounded-2xl border border-border bg-[#050505] p-5 space-y-3 shadow-2xl">
              <div className="flex items-center justify-between border-b border-border/40 pb-2">
                <div className="flex items-center gap-3 text-xs">
                  <span className="font-semibold text-foreground">Execution Output</span>
                  <span className={cn(
                    'px-2 py-0.5 rounded-full text-[10px] font-semibold border flex items-center gap-1',
                    lastResult.exit_code === 0
                      ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
                      : 'bg-rose-500/10 text-rose-400 border-rose-500/30'
                  )}>
                    {lastResult.exit_code === 0 ? <CheckCircle2 size={11} /> : <XCircle size={11} />}
                    Exit Code: {lastResult.exit_code}
                  </span>
                </div>
                <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground font-mono">
                  <Clock size={12} />
                  <span>{lastResult.duration_ms} ms</span>
                </div>
              </div>

              {/* Stdout */}
              {lastResult.stdout && (
                <div>
                  <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider block mb-1">Standard Output (stdout)</span>
                  <pre className="p-3 bg-[#0d0d0d] border border-border/40 rounded-xl font-mono text-xs text-emerald-300/90 whitespace-pre-wrap overflow-x-auto max-h-64 leading-relaxed">
                    {lastResult.stdout}
                  </pre>
                </div>
              )}

              {/* Stderr */}
              {lastResult.stderr && (
                <div>
                  <span className="text-[10px] font-semibold text-rose-400 uppercase tracking-wider block mb-1">Standard Error (stderr)</span>
                  <pre className="p-3 bg-rose-950/20 border border-rose-500/30 rounded-xl font-mono text-xs text-rose-300/90 whitespace-pre-wrap overflow-x-auto max-h-48 leading-relaxed">
                    {lastResult.stderr}
                  </pre>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* ── SUB-TAB 2: GUEST OS USER MANAGEMENT & SYNC ── */}
      {subTab === 'users' && (
        <div className="space-y-4">
          <div className="rounded-2xl border border-border bg-card p-5 space-y-4 shadow-xl">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
              <div>
                <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Users size={16} className="text-primary" />
                  Guest OS User Account Synchronizer
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Manage user accounts directly inside '{vmName}' OS and keep Portal and OS records synchronized.
                </p>
              </div>

              <div className="flex items-center gap-2">
                <button
                  onClick={handleSyncUsers}
                  disabled={loadingUsers}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-muted/40 hover:bg-muted text-xs font-semibold text-foreground transition-all"
                >
                  <RefreshCw size={13} className={cn(loadingUsers && 'animate-spin')} />
                  Sync with VM OS
                </button>
                <button
                  onClick={() => setShowAddUserModal(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary hover:bg-primary/90 text-primary-foreground text-xs font-semibold transition-all shadow-sm"
                >
                  <UserPlus size={13} />
                  Add OS User
                </button>
              </div>
            </div>

            {/* Users Table */}
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-border text-muted-foreground font-semibold">
                    <th className="py-2.5 px-3">Username</th>
                    <th className="py-2.5 px-3">Full Name</th>
                    <th className="py-2.5 px-3">Privilege / Role</th>
                    <th className="py-2.5 px-3">Status</th>
                    <th className="py-2.5 px-3 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border/50">
                  {users.map((u) => (
                    <tr key={u.username} className="hover:bg-muted/30 transition-colors">
                      <td className="py-3 px-3 font-semibold text-foreground flex items-center gap-2">
                        <div className="w-6 h-6 rounded-full bg-primary/10 border border-primary/30 flex items-center justify-center text-[10px] font-bold text-primary">
                          {u.username[0]?.toUpperCase()}
                        </div>
                        {u.username}
                      </td>
                      <td className="py-3 px-3 text-muted-foreground">{u.full_name || '—'}</td>
                      <td className="py-3 px-3">
                        <span className={cn(
                          'px-2 py-0.5 rounded-full text-[10px] font-semibold border',
                          u.is_admin
                            ? 'bg-amber-500/10 text-amber-400 border-amber-500/30'
                            : 'bg-blue-500/10 text-blue-400 border-blue-500/30'
                        )}>
                          {u.is_admin ? 'Administrator / Root' : 'Standard User'}
                        </span>
                      </td>
                      <td className="py-3 px-3">
                        <span className={cn(
                          'px-2 py-0.5 rounded-full text-[10px] font-semibold border',
                          !u.is_disabled
                            ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
                            : 'bg-rose-500/10 text-rose-400 border-rose-500/30'
                        )}>
                          {!u.is_disabled ? 'Active & Synced' : 'Disabled'}
                        </span>
                      </td>
                      <td className="py-3 px-3 text-right">
                        <button
                          onClick={() => {
                            setResetTargetUser(u.username)
                            setResetNewPassword('')
                          }}
                          className="flex items-center gap-1 ml-auto px-2.5 py-1 rounded-md border border-border hover:bg-muted text-[11px] font-medium text-muted-foreground hover:text-foreground transition-colors"
                        >
                          <Key size={11} />
                          Reset Password
                        </button>
                      </td>
                    </tr>
                  ))}
                  {users.length === 0 && !loadingUsers && (
                    <tr>
                      <td colSpan={5} className="py-8 text-center text-muted-foreground italic">
                        No OS user accounts detected. Click "Sync with VM OS" to scan guest user database.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* ── MODAL 1: ADD OS USER ── */}
      {showAddUserModal && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-card border border-border rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
            <div className="flex items-center justify-between border-b border-border pb-3">
              <h4 className="text-sm font-semibold text-foreground flex items-center gap-2">
                <UserPlus size={16} className="text-primary" />
                Add OS User to '{vmName}'
              </h4>
              <button onClick={() => setShowAddUserModal(false)} className="text-muted-foreground hover:text-foreground">✕</button>
            </div>

            <form onSubmit={handleCreateUser} className="space-y-3">
              <div>
                <label className="text-xs font-semibold text-muted-foreground block mb-1">Username</label>
                <input
                  type="text"
                  required
                  value={newUsername}
                  onChange={(e) => setNewUsername(e.target.value)}
                  placeholder="e.g. devadmin"
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-muted-foreground block mb-1">Full Name</label>
                <input
                  type="text"
                  value={newFullName}
                  onChange={(e) => setNewFullName(e.target.value)}
                  placeholder="e.g. Development Administrator"
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-muted-foreground block mb-1">Initial Password</label>
                <input
                  type="password"
                  required
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="Set strong account password..."
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>

              <div className="flex items-center gap-2 pt-1">
                <input
                  type="checkbox"
                  id="isAdminCheck"
                  checked={newIsAdmin}
                  onChange={(e) => setNewIsAdmin(e.target.checked)}
                  className="rounded border-border text-primary focus:ring-primary"
                />
                <label htmlFor="isAdminCheck" className="text-xs font-semibold text-foreground cursor-pointer">
                  Grant Administrator / Root privileges
                </label>
              </div>

              <div className="flex justify-end gap-2 pt-3">
                <button
                  type="button"
                  onClick={() => setShowAddUserModal(false)}
                  className="px-4 py-2 rounded-xl text-xs font-semibold border border-border hover:bg-muted text-muted-foreground"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={creatingUser}
                  className="px-4 py-2 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:bg-primary/90 shadow-md"
                >
                  {creatingUser ? 'Creating in OS...' : 'Create & Sync User'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* ── MODAL 2: RESET PASSWORD ── */}
      {resetTargetUser && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-card border border-border rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
            <div className="flex items-center justify-between border-b border-border pb-3">
              <h4 className="text-sm font-semibold text-foreground flex items-center gap-2">
                <Lock size={16} className="text-amber-400" />
                Reset Password for '{resetTargetUser}'
              </h4>
              <button onClick={() => setResetTargetUser(null)} className="text-muted-foreground hover:text-foreground">✕</button>
            </div>

            <div className="space-y-3">
              <div>
                <label className="text-xs font-semibold text-muted-foreground block mb-1">New Account Password</label>
                <input
                  type="password"
                  value={resetNewPassword}
                  onChange={(e) => setResetNewPassword(e.target.value)}
                  placeholder="Enter new password..."
                  className="w-full bg-background border border-border rounded-lg px-3 py-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>

              <div className="flex justify-end gap-2 pt-3">
                <button
                  onClick={() => setResetTargetUser(null)}
                  className="px-4 py-2 rounded-xl text-xs font-semibold border border-border hover:bg-muted text-muted-foreground"
                >
                  Cancel
                </button>
                <button
                  onClick={handleResetPassword}
                  disabled={updatingPassword || !resetNewPassword}
                  className="px-4 py-2 rounded-xl text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white shadow-md"
                >
                  {updatingPassword ? 'Updating OS...' : 'Update Password in VM'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
