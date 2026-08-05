import { Component, ErrorInfo, ReactNode } from 'react'

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  }

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught React render error:', error, errorInfo)
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center min-h-[70vh] p-6 text-foreground">
          <div className="max-w-md w-full rounded-2xl bg-card border border-destructive/40 shadow-2xl p-6 space-y-4 text-center">
            <div className="w-12 h-12 rounded-full bg-destructive/15 text-destructive flex items-center justify-center mx-auto text-xl font-bold">
              !
            </div>
            <h2 className="text-xl font-bold tracking-tight text-foreground">Component Render Error</h2>
            <p className="text-sm text-muted-foreground">
              An unexpected render error occurred in this view.
            </p>
            <div className="p-3 rounded-xl bg-muted font-mono text-xs text-left text-destructive whitespace-pre-wrap break-all max-h-36 overflow-y-auto border border-border">
              {this.state.error?.toString()}
            </div>
            <button
              onClick={() => {
                this.setState({ hasError: false, error: null })
                window.location.href = '/vms'
              }}
              className="w-full py-2.5 text-sm font-medium bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors shadow"
            >
              Return to Virtual Machines
            </button>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}
