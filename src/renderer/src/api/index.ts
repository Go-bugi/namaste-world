import { getTauriVersion, getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ResultAsync } from 'neverthrow';
import type { AgentKind } from '@renderer/types/bindings/AgentKind';
import type { Detection } from '@renderer/types/bindings/Detection';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { HookEvent } from '@renderer/types/bindings/HookEvent';
import type { HookInstallation } from '@renderer/types/bindings/HookInstallation';
import type { LoginStatus } from '@renderer/types/bindings/LoginStatus';
import type { ModelList } from '@renderer/types/bindings/ModelList';
import type { OnboardingError } from '@renderer/types/bindings/OnboardingError';
import type { Selection } from '@renderer/types/bindings/Selection';
import type { ShellKind } from '@renderer/types/bindings/ShellKind';
import type { ShellSetup } from '@renderer/types/bindings/ShellSetup';
import { HOOK_EVENT, HOOK_INSTALLATION } from '@renderer/types/ipc';

/** Rust 명령 에러가 아닌 IPC 실패. 없는 명령이나 인자 모양 불일치처럼 Tauri가 문자열로 reject할 때 쓴다. */
export type IpcError = { readonly kind: 'ipc'; readonly message: string };

/**
 * reject 값을 에러 유니온으로 바꾼다.
 * Rust 명령 에러는 `kind`가 있는 객체(ts-rs가 만든 타입)로 오고, 그 밖의 실패는 IpcError가 된다.
 */
const toCallError = <E extends { readonly kind: string }>(e: unknown): E | IpcError =>
  typeof e === 'object' && e !== null && 'kind' in e && typeof e.kind === 'string'
    ? (e as E)
    : { kind: 'ipc', message: e instanceof Error ? e.message : String(e) };

/** reject될 수 있는 호출을 ResultAsync로 감싼다. E는 그 명령이 돌려주는 Rust 에러 타입이다. */
const call = <T, E extends { readonly kind: string } = never>(
  promise: Promise<T>,
): ResultAsync<T, E | IpcError> => ResultAsync.fromPromise(promise, toCallError<E>);

/**
 * Rust 쪽과 통신하는 함수는 여기에만 둔다. 컴포넌트에서 invoke를 직접 부르지 않는다.
 * 명령 이름과 인자는 src-tauri/src/commands/의 #[tauri::command]와 맞춘다.
 */
export const api = {
  ping: (): ResultAsync<void, IpcError> => call(invoke('ping')),
  getAppVersion: (): ResultAsync<string, IpcError> => call(getVersion()),
  getTauriVersion: (): ResultAsync<string, IpcError> => call(getTauriVersion()),
  /** 보관한 에이전트 감지 결과. 앱 시작 감지가 아직이면 끝날 때까지 기다린다. */
  getAgentDetection: (): ResultAsync<Detection, IpcError> => call(invoke('agent_detection')),
  /** 에이전트를 새로 감지한다. 멈춘 WSL 배포판도 켜서 확인한다. */
  redetectAgents: (): ResultAsync<Detection, IpcError> => call(invoke('redetect_agents')),
  /** 보관한 훅 설치 결과. 첫 설치가 아직이면 끝날 때까지 기다린다. */
  getHookInstallation: (): ResultAsync<HookInstallation, IpcError> =>
    call(invoke('hook_installation')),
  /** 훅 이벤트를 받을 때마다 handler를 부른다. 돌려받은 함수로 구독을 끝낸다. */
  onHookEvent: (handler: (event: HookEvent) => void): ResultAsync<UnlistenFn, IpcError> =>
    call(listen<HookEvent>(HOOK_EVENT, (message) => handler(message.payload))),
  /** 훅 설치가 끝날 때마다(다시 감지 뒤 포함) handler를 부른다. */
  onHookInstallation: (
    handler: (installation: HookInstallation) => void,
  ): ResultAsync<UnlistenFn, IpcError> =>
    call(listen<HookInstallation>(HOOK_INSTALLATION, (message) => handler(message.payload))),
  /** 한 에이전트와 실행 환경의 모델 목록. 조회가 실패하거나 5초를 넘기면 고정 목록이 온다. */
  getAgentModels: (agent: AgentKind, environment: Environment): ResultAsync<ModelList, IpcError> =>
    call(invoke('agent_models', { agent, environment })),
  /** 보관한 감지 결과로 훅을 다시 설치하고 새 설치 결과를 준다. 끝나면 설치 결과 이벤트도 온다. */
  reinstallHooks: (): ResultAsync<HookInstallation, IpcError> => call(invoke('reinstall_hooks')),
  /** 한 에이전트와 실행 환경의 로그인 상태. 알아내지 못하면 `failed`에 이유가 담겨 온다. */
  getAgentLoginStatus: (
    agent: AgentKind,
    environment: Environment,
  ): ResultAsync<LoginStatus, IpcError> =>
    call(invoke('agent_login_status', { agent, environment })),
  /** 지난번에 고른 에이전트와 모델. 없으면 undefined다. */
  getSavedSelection: (): ResultAsync<Selection | undefined, IpcError> =>
    call(invoke<Selection | null>('saved_selection')).map((saved) => saved ?? undefined),
  /** 작은 창을 연 뒤 고른 값을 저장하고 시작 설정 창을 닫는다. 작은 창을 열지 못하면 아무것도 바꾸지 않는다. */
  finishOnboarding: (selection: Selection): ResultAsync<void, OnboardingError | IpcError> =>
    call<void, OnboardingError>(invoke('finish_onboarding', { selection })),
  /** 셸 선택 단계의 선택지와 미리 고를 셸. 셸을 고르지 않는 OS(macOS)는 undefined다. */
  getShellSetup: (): ResultAsync<ShellSetup | undefined, IpcError> =>
    call(invoke<ShellSetup | null>('shell_setup')).map((setup) => setup ?? undefined),
  /** 고른 셸을 앱 데이터 폴더에 바로 저장한다. 감지는 하지 않는다. */
  setShell: (shell: ShellKind): ResultAsync<void, IpcError> => call(invoke('set_shell', { shell })),
  /**
   * 고른 셸의 환경으로 감지한다. 처음 부르면 앱 시작 감지가 되고, 마지막 감지와 셸이 같으면 보관한 결과가 온다.
   * 셸이 다르면 다시 감지하고(멈춘 WSL 배포판은 켜지 않음) 훅을 다시 설치한다.
   */
  detectWithShell: (shell: ShellKind): ResultAsync<Detection, IpcError> =>
    call(invoke('detect_with_shell', { shell })),
  /** 누르고 있는 마우스로 지금 창을 끌기 시작한다. 테두리 없는 작은 창에서 쓴다. */
  startWindowDrag: (): ResultAsync<void, IpcError> => call(getCurrentWindow().startDragging()),
};
