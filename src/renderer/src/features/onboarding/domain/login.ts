import type { AgentKind } from '@renderer/types/bindings/AgentKind';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { LoginFailure } from '@renderer/types/bindings/LoginFailure';
import type { LoginMethod } from '@renderer/types/bindings/LoginMethod';
import type { LoginStatus } from '@renderer/types/bindings/LoginStatus';

/** 로그인 확인 단계의 상태. key는 확인한 선택지의 `agentEnvironmentKey`다. */
export type LoginState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'checking'; readonly key: string; readonly requestId: number }
  | { readonly kind: 'checked'; readonly key: string; readonly status: LoginStatus };

export type LoginEvent =
  | { readonly type: 'checkStarted'; readonly key: string; readonly requestId: number }
  | { readonly type: 'checked'; readonly requestId: number; readonly status: LoginStatus };

export const initialLoginState: LoginState = { kind: 'idle' };

/** 확인 시작과 결과를 반영한다. requestId가 지금 확인과 다른 결과(이전 항목, 이전 확인)는 버린다. */
export function loginReducer(state: LoginState, event: LoginEvent): LoginState {
  switch (event.type) {
    case 'checkStarted':
      return { kind: 'checking', key: event.key, requestId: event.requestId };
    case 'checked':
      return state.kind === 'checking' && state.requestId === event.requestId
        ? { kind: 'checked', key: state.key, status: event.status }
        : state;
  }
}

/** key 항목의 확인 결과가 로그인됨인가. */
export function isLoggedIn(state: LoginState, key: string | undefined): boolean {
  return (
    key !== undefined &&
    state.kind === 'checked' &&
    state.key === key &&
    state.status.kind === 'loggedIn'
  );
}

/** 다시 확인 버튼을 누를 수 있는가. 확인 중이면 false. */
export function canRecheck(state: LoginState): boolean {
  return state.kind !== 'checking';
}

/** 에이전트마다 사용자가 터미널에서 실행할 로그인 명령. */
const LOGIN_COMMAND: Record<AgentKind, string> = {
  claude: 'claude auth login',
  codex: 'codex login',
};

const METHOD_LABEL: Record<LoginMethod, string> = {
  subscription: '구독',
  apiKey: 'API 키',
  cloud: '클라우드',
  other: '기타',
};

const FAILURE_LABEL: Record<LoginFailure['kind'], string> = {
  notInstalled: '감지 결과에 설치된 항목이 없습니다. 감지 단계에서 다시 감지해 주세요',
  launchFailed: 'CLI를 실행하지 못했습니다',
  timeout: '시간 초과',
  unreadableOutput:
    'CLI 응답을 알아볼 수 없습니다. CLI를 최신 버전으로 업데이트한 뒤 다시 확인해 주세요',
};

/** 로그인 안내. 터미널에서 실행할 명령과 실행할 곳. */
export type LoginGuide = { readonly command: string; readonly where: string };

/**
 * 로그인 안 됨일 때만 안내를 준다. command는 Claude Code `claude auth login`, Codex `codex login`이다.
 * WSL이면 where에 배포판 이름이 든 터미널 문구를 준다. FR-START-009
 */
export function loginGuide(
  agent: AgentKind,
  environment: Environment,
  status: LoginStatus,
): LoginGuide | undefined {
  if (status.kind !== 'loggedOut') return undefined;
  return {
    command: LOGIN_COMMAND[agent],
    where: environment.kind === 'wsl' ? `WSL ${environment.distro} 터미널` : '터미널',
  };
}

/** 로그인 상태 문구. 로그인됨(방식), 로그인 안 됨, 확인 실패와 이유. */
export function loginStatusLabel(status: LoginStatus): string {
  switch (status.kind) {
    case 'loggedIn':
      return status.method === undefined ? '로그인됨' : `로그인됨 (${METHOD_LABEL[status.method]})`;
    case 'loggedOut':
      return '로그인 안 됨';
    case 'failed':
      return `확인 실패: ${FAILURE_LABEL[status.reason.kind]}`;
  }
}
