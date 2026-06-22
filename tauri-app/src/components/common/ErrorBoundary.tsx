import { Component, type ReactNode, type ErrorInfo } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return (
        <div className="p-6 text-center">
          <h2 className="text-lg font-bold text-red-600 mb-2">页面渲染异常</h2>
          <p className="text-sm text-gray-500 mb-4 font-mono break-all">
            {this.state.error.message}
          </p>
          <button
            onClick={() => this.setState({ error: null })}
            className="text-sm text-xhs hover:underline"
          >
            重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
