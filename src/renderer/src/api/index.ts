import { getTauriVersion, getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { ResultAsync } from 'neverthrow';
import type { Detection } from '@renderer/types/bindings/Detection';

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
};
