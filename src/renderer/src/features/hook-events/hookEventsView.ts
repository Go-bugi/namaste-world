import type { HookEvent } from '@renderer/types/bindings/HookEvent';
import type { HookInstallation } from '@renderer/types/bindings/HookInstallation';
import type { InstallEntry } from '@renderer/types/bindings/InstallEntry';
import type { InstallFailure } from '@renderer/types/bindings/InstallFailure';
import type { InstallStatus } from '@renderer/types/bindings/InstallStatus';
import {
  agentEnvironmentKey,
  agentLabel,
  environmentLabel,
  type AgentLabel,
} from '@renderer/shared/lib/agentLabels';

/** 화면에 남기는 이벤트 수. 넘으면 오래된 것부터 뺀다. */
export const EVENT_LIMIT = 200;

/** 새 이벤트를 끝에 붙이고 limit개를 넘으면 앞(오래된 것)부터 뺀다. 입력 배열은 바꾸지 않는다. */
export function appendEvent(
  list: readonly HookEvent[],
  event: HookEvent,
  limit: number = EVENT_LIMIT,
): readonly HookEvent[] {
  const all = [...list, event];
  return all.length > limit ? all.slice(all.length - limit) : all;
}

/** 이벤트 한 줄. time은 'HH:MM:SS'(로컬 시각)이다. */
export type EventRow = {
  readonly key: string;
  readonly time: string;
  readonly agentLabel: AgentLabel;
  readonly event: string;
};

/** 이벤트를 화면 한 줄로 바꾼다. index는 목록 안 위치로, key를 만드는 데 쓴다. */
export function toEventRow(event: HookEvent, index: number): EventRow {
  return {
    key: `${index}:${event.receivedAt}`,
    time: clockTime(new Date(event.receivedAt)),
    agentLabel: agentLabel(event.agent),
    event: event.event,
  };
}

const pad2 = (n: number): string => String(n).padStart(2, '0');

function clockTime(date: Date): string {
  return `${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`;
}

/** 설치 결과 한 줄. detail은 실패 이유 문구다. */
export type InstallRow = {
  readonly key: string;
  readonly agentLabel: AgentLabel;
  readonly environmentLabel: string;
  readonly statusLabel: '새로 설치함' | '이미 설치됨' | '실패';
  readonly detail?: string;
};

/** 설치 결과를 entries 순서 그대로 화면 줄로 바꾼다. */
export function toInstallRows(installation: HookInstallation): readonly InstallRow[] {
  return installation.entries.map(toInstallRow);
}

function toInstallRow({ agent, environment, status }: InstallEntry): InstallRow {
  return {
    key: agentEnvironmentKey(agent, environment),
    agentLabel: agentLabel(agent),
    environmentLabel: environmentLabel(environment),
    statusLabel: installStatusLabel(status),
    ...(status.status === 'failed' ? { detail: installFailureLabel(status.reason) } : {}),
  };
}

function installStatusLabel(status: InstallStatus): InstallRow['statusLabel'] {
  switch (status.status) {
    case 'installed':
      return '새로 설치함';
    case 'alreadyInstalled':
      return '이미 설치됨';
    case 'failed':
      return '실패';
  }
}

/** 설치 실패 이유 문구. */
export function installFailureLabel(reason: InstallFailure): string {
  switch (reason.kind) {
    case 'invalidConfig':
      return `설정 파일을 해석할 수 없습니다: ${reason.file}`;
    case 'notWritable':
      return `설정 파일에 쓸 권한이 없습니다: ${reason.file}`;
    case 'codexHooksDisabled':
      return `Codex 훅 기능이 꺼져 있습니다([features] hooks = false): ${reason.file}`;
    case 'io':
      return reason.message === '' ? '파일을 읽거나 쓰지 못했습니다' : reason.message;
  }
}
