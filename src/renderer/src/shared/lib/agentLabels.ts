import type { AgentKind } from '@renderer/types/bindings/AgentKind';
import type { Environment } from '@renderer/types/bindings/Environment';

/** 에이전트 이름표. */
export type AgentLabel = 'Claude Code' | 'Codex';

/** 에이전트를 화면에 보이는 이름으로 바꾼다. */
export function agentLabel(agent: AgentKind): AgentLabel {
  switch (agent) {
    case 'claude':
      return 'Claude Code';
    case 'codex':
      return 'Codex';
  }
}

/** 실행 환경을 화면에 보이는 이름으로 바꾼다. 'macOS' | 'Windows' | `WSL · ${distro}` */
export function environmentLabel(environment: Environment): string {
  switch (environment.kind) {
    case 'macos':
      return 'macOS';
    case 'windows':
      return 'Windows';
    case 'wsl':
      return `WSL · ${environment.distro}`;
  }
}

/** 에이전트와 실행 환경 조합의 React key. `${agent}:${환경 kind}:${distro ?? ''}` */
export function agentEnvironmentKey(agent: AgentKind, environment: Environment): string {
  return `${agent}:${environment.kind}:${environment.kind === 'wsl' ? environment.distro : ''}`;
}
