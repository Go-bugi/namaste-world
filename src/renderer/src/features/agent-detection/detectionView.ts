import type { AgentEntry } from '@renderer/types/bindings/AgentEntry';
import type { AgentStatus } from '@renderer/types/bindings/AgentStatus';
import type { Detection } from '@renderer/types/bindings/Detection';
import type { FailureReason } from '@renderer/types/bindings/FailureReason';
import {
  agentEnvironmentKey,
  agentLabel,
  environmentLabel,
  type AgentLabel,
} from '@renderer/shared/lib/agentLabels';

/** 목록 화면의 한 줄. label이 붙은 값은 사용자에게 그대로 보이는 문구다. */
export type DetectionRow = {
  /** React key. `${agent}:${환경 kind}:${distro ?? ''}` */
  readonly key: string;
  readonly agentLabel: AgentLabel;
  /** 'macOS' | 'Windows' | `WSL · ${distro}` */
  readonly environmentLabel: string;
  readonly statusLabel: '설치됨' | '설치되지 않음' | '확인 실패' | '멈춤';
  /** 설치됨: `${version} · ${path}`, 확인 실패: 이유 문구, 멈춤: 다시 감지 안내, 설치되지 않음: 없음 */
  readonly detail?: string;
};

/** 멈춘 배포판 줄에 붙는 안내 문구. */
export const STOPPED_DETAIL = '다시 감지하면 배포판을 켜서 확인합니다';

/** 감지 결과를 entries 순서 그대로 화면 줄로 바꾼다. */
export function toRows(detection: Detection): readonly DetectionRow[] {
  return detection.entries.map(toRow);
}

function toRow({ agent, environment, status }: AgentEntry): DetectionRow {
  const detail = detailOf(status);
  return {
    key: agentEnvironmentKey(agent, environment),
    agentLabel: agentLabel(agent),
    environmentLabel: environmentLabel(environment),
    statusLabel: statusLabel(status),
    ...(detail === undefined ? {} : { detail }),
  };
}

function statusLabel(status: AgentStatus): DetectionRow['statusLabel'] {
  switch (status.status) {
    case 'installed':
      return '설치됨';
    case 'notInstalled':
      return '설치되지 않음';
    case 'failed':
      return '확인 실패';
    case 'stopped':
      return '멈춤';
  }
}

function detailOf(status: AgentStatus): string | undefined {
  switch (status.status) {
    case 'installed':
      return `${status.version} · ${status.path}`;
    case 'notInstalled':
      return undefined;
    case 'failed':
      return failureLabel(status.reason);
    case 'stopped':
      return STOPPED_DETAIL;
  }
}

/** 실패 이유 문구. timeout → '시간 초과', versionUnreadable → '버전 확인 실패', environmentError → message */
export function failureLabel(reason: FailureReason): string {
  switch (reason.kind) {
    case 'timeout':
      return '시간 초과';
    case 'versionUnreadable':
      return '버전 확인 실패';
    case 'environmentError':
      return reason.message;
  }
}

/** 화면 상태. result가 없으면 처음 감지 중이다. */
export type PanelState = { readonly result?: Detection; readonly detecting: boolean };

export type PanelEvent =
  | { readonly type: 'redetectStarted' }
  | { readonly type: 'detected'; readonly result: Detection }
  | { readonly type: 'failed' };

/** 앱 시작 감지 결과를 기다리는 처음 상태. */
export const initialPanelState: PanelState = { detecting: true };

/**
 * 화면 상태 전이.
 * redetectStarted → 이전 result를 두고 detecting: true / detected → result 교체, detecting: false /
 * failed → result 유지, detecting: false
 */
export function panelReducer(state: PanelState, event: PanelEvent): PanelState {
  switch (event.type) {
    case 'redetectStarted':
      return { ...state, detecting: true };
    case 'detected':
      return { result: event.result, detecting: false };
    case 'failed':
      return { ...state, detecting: false };
  }
}

/** 다시 감지 버튼을 누를 수 있는가. 감지 중이면 false다. */
export function canRedetect(state: PanelState): boolean {
  return !state.detecting;
}
