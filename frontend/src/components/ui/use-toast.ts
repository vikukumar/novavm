import { useState, useCallback } from 'react'

interface Toast {
  id: string
  title?: string
  description?: string
  variant?: 'default' | 'destructive'
}

let toastState: Toast[] = []
let listeners: Array<(toasts: Toast[]) => void> = []

function emitChange() {
  for (const listener of listeners) {
    listener(toastState)
  }
}

export function toast(t: Omit<Toast, 'id'>) {
  const id = crypto.randomUUID()
  toastState = [...toastState, { ...t, id }]
  emitChange()
  setTimeout(() => {
    toastState = toastState.filter((t) => t.id !== id)
    emitChange()
  }, 4000)
}

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>(toastState)

  useCallback(() => {
    listeners.push(setToasts)
    return () => {
      listeners = listeners.filter((l) => l !== setToasts)
    }
  }, [])()

  return { toasts, toast }
}
