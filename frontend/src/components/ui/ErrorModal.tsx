import { AlertTriangle, Copy, X } from 'lucide-react'
import { useState } from 'react'

interface ErrorModalProps {
  isOpen: boolean
  title?: string
  error: Error | string | null
  onClose: () => void
}

export function ErrorModal({ isOpen, title = 'Operation Failed', error, onClose }: ErrorModalProps) {
  const [copied, setCopied] = useState(false)

  if (!isOpen || !error) return null

  const errorMessage = typeof error === 'string' ? error : error.message
  const errorCode = (error && typeof error === 'object' && 'code' in error) ? (error as { code: string }).code : 'UNKNOWN_ERROR'

  const handleCopy = () => {
    const fullText = `[NovaVM Error - ${errorCode}]\nTitle: ${title}\nMessage: ${errorMessage}`
    navigator.clipboard.writeText(fullText)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
      <div className="relative w-full max-w-lg rounded-2xl bg-card border border-destructive/30 shadow-2xl p-6 overflow-hidden">
        {/* Header */}
        <div className="flex items-start gap-4 mb-4">
          <div className="flex-shrink-0 w-11 h-11 rounded-xl bg-destructive/15 flex items-center justify-center text-destructive">
            <AlertTriangle size={22} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="text-lg font-bold tracking-tight text-foreground">{title}</h3>
              <span className="px-2 py-0.5 text-xs font-mono font-medium rounded bg-destructive/20 text-destructive border border-destructive/30">
                {errorCode}
              </span>
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              An error occurred while communicating with the NovaVM hypervisor engine.
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Details Box */}
        <div className="p-3.5 rounded-xl bg-muted/60 border border-border text-xs font-mono text-foreground/90 whitespace-pre-wrap break-words max-h-48 overflow-y-auto mb-6">
          {errorMessage}
        </div>

        {/* Action Buttons */}
        <div className="flex items-center justify-between">
          <button
            onClick={handleCopy}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-muted-foreground hover:text-foreground rounded-lg bg-muted hover:bg-accent transition-colors"
          >
            <Copy size={14} />
            {copied ? 'Copied to Clipboard!' : 'Copy Error Details'}
          </button>
          <button
            onClick={onClose}
            className="px-5 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors shadow-sm"
          >
            Dismiss
          </button>
        </div>
      </div>
    </div>
  )
}
