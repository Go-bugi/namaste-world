import { fc, test } from '@fast-check/vitest';
import type { AgentEntry } from '@renderer/types/bindings/AgentEntry';
import type { AgentStatus } from '@renderer/types/bindings/AgentStatus';
import type { Detection } from '@renderer/types/bindings/Detection';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { FailureReason } from '@renderer/types/bindings/FailureReason';
import { describe, expect, it } from 'vitest';
import {
  STOPPED_DETAIL,
  canRedetect,
  failureLabel,
  initialPanelState,
  panelReducer,
  toRows,
  type PanelState,
} from './detectionView';

const windows: Environment = { kind: 'windows' };
const macos: Environment = { kind: 'macos' };
const ubuntu: Environment = { kind: 'wsl', distro: 'Ubuntu' };

const entry = (
  agent: AgentEntry['agent'],
  environment: Environment,
  status: AgentStatus,
): AgentEntry => ({
  agent,
  environment,
  status,
});

const detection = (...entries: AgentEntry[]): Detection => ({ entries });

const environmentArb: fc.Arbitrary<Environment> = fc.oneof(
  fc.constant<Environment>({ kind: 'macos' }),
  fc.constant<Environment>({ kind: 'windows' }),
  fc.string({ minLength: 1 }).map((distro): Environment => ({ kind: 'wsl', distro })),
);

const reasonArb: fc.Arbitrary<FailureReason> = fc.oneof(
  fc.constant<FailureReason>({ kind: 'timeout' }),
  fc.constant<FailureReason>({ kind: 'versionUnreadable' }),
  fc.string().map((message): FailureReason => ({ kind: 'environmentError', message })),
);

const statusArb: fc.Arbitrary<AgentStatus> = fc.oneof(
  fc
    .record({ path: fc.string(), version: fc.string() })
    .map(({ path, version }): AgentStatus => ({ status: 'installed', path, version })),
  fc.constant<AgentStatus>({ status: 'notInstalled' }),
  fc.constant<AgentStatus>({ status: 'stopped' }),
  reasonArb.map((reason): AgentStatus => ({ status: 'failed', reason })),
);

const entryArb: fc.Arbitrary<AgentEntry> = fc.record({
  agent: fc.constantFrom<AgentEntry['agent']>('claude', 'codex'),
  environment: environmentArb,
  status: statusArb,
});

const detectionArb: fc.Arbitrary<Detection> = fc
  .array(entryArb, { maxLength: 8 })
  .map((entries) => ({ entries }));

const panelStateArb: fc.Arbitrary<PanelState> = fc.oneof(
  fc.boolean().map((detecting): PanelState => ({ detecting })),
  fc.record({ result: detectionArb, detecting: fc.boolean() }),
);

describe('failureLabel', () => {
  it('FR-AGENT-008 시나리오 1: 시간 초과 이유를 문구로 보여 준다', () => {
    expect(failureLabel({ kind: 'timeout' })).toBe('시간 초과');
  });

  it('FR-AGENT-003 시나리오 2: 버전 확인 실패 이유를 문구로 보여 준다', () => {
    expect(failureLabel({ kind: 'versionUnreadable' })).toBe('버전 확인 실패');
  });

  test.prop([fc.string()])('FR-AGENT-005: 환경 오류는 그 메시지를 이유로 보여 준다', (message) => {
    expect(failureLabel({ kind: 'environmentError', message })).toBe(message);
  });
});

describe('toRows', () => {
  it('FR-AGENT-008 시나리오 1: Windows Codex 줄은 버전과 경로를, WSL Ubuntu 줄은 확인 실패와 이유를 보여 준다', () => {
    const rows = toRows(
      detection(
        entry('codex', windows, {
          status: 'installed',
          path: 'C:\\Users\\u\\AppData\\Roaming\\npm\\codex.cmd',
          version: '0.146.0',
        }),
        entry('claude', ubuntu, { status: 'failed', reason: { kind: 'timeout' } }),
        entry('codex', ubuntu, { status: 'failed', reason: { kind: 'timeout' } }),
      ),
    );

    expect(rows).toEqual([
      {
        key: 'codex:windows:',
        agentLabel: 'Codex',
        environmentLabel: 'Windows',
        statusLabel: '설치됨',
        detail: '0.146.0 · C:\\Users\\u\\AppData\\Roaming\\npm\\codex.cmd',
      },
      {
        key: 'claude:wsl:Ubuntu',
        agentLabel: 'Claude Code',
        environmentLabel: 'WSL · Ubuntu',
        statusLabel: '확인 실패',
        detail: '시간 초과',
      },
      {
        key: 'codex:wsl:Ubuntu',
        agentLabel: 'Codex',
        environmentLabel: 'WSL · Ubuntu',
        statusLabel: '확인 실패',
        detail: '시간 초과',
      },
    ]);
  });

  it('FR-AGENT-008: macOS의 설치되지 않은 에이전트 줄에는 detail이 없다', () => {
    const [row] = toRows(detection(entry('claude', macos, { status: 'notInstalled' })));

    expect(row).toEqual({
      key: 'claude:macos:',
      agentLabel: 'Claude Code',
      environmentLabel: 'macOS',
      statusLabel: '설치되지 않음',
    });
    expect(row?.detail).toBeUndefined();
  });

  it('FR-AGENT-008: 멈춘 배포판 줄에는 다시 감지하면 배포판을 켠다는 안내가 붙는다', () => {
    const rows = toRows(
      detection(
        entry('claude', ubuntu, { status: 'stopped' }),
        entry('codex', ubuntu, { status: 'stopped' }),
      ),
    );

    expect(rows.map((r) => [r.key, r.statusLabel, r.detail])).toEqual([
      ['claude:wsl:Ubuntu', '멈춤', STOPPED_DETAIL],
      ['codex:wsl:Ubuntu', '멈춤', STOPPED_DETAIL],
    ]);
    expect(STOPPED_DETAIL).toBe('다시 감지하면 배포판을 켜서 확인합니다');
  });

  it('FR-AGENT-008: 환경 오류로 확인 실패한 줄은 그 메시지를 이유로 보여 준다', () => {
    const [row] = toRows(
      detection(
        entry('codex', ubuntu, {
          status: 'failed',
          reason: { kind: 'environmentError', message: 'wsl.exe 실패' },
        }),
      ),
    );

    expect(row?.statusLabel).toBe('확인 실패');
    expect(row?.detail).toBe('wsl.exe 실패');
  });

  it('FR-AGENT-002 시나리오 1: 같은 에이전트가 두 환경에 있으면 환경마다 따로 한 줄씩 나온다', () => {
    const installed: AgentStatus = {
      status: 'installed',
      path: '/home/user/.local/bin/codex',
      version: '0.146.0',
    };
    const rows = toRows(
      detection(entry('codex', windows, installed), entry('codex', ubuntu, installed)),
    );

    expect(rows.map((r) => [r.key, r.environmentLabel])).toEqual([
      ['codex:windows:', 'Windows'],
      ['codex:wsl:Ubuntu', 'WSL · Ubuntu'],
    ]);
  });

  // 속성: 줄은 entries와 한 대 한 대응이고 순서도 같다
  test.prop([detectionArb])(
    'FR-AGENT-008: 에이전트와 실행 환경마다 한 줄씩 entries 순서대로 낸다',
    (d) => {
      const rows = toRows(d);

      expect(rows).toHaveLength(d.entries.length);
      expect(rows.map((r) => r.agentLabel)).toEqual(
        d.entries.map((e) => (e.agent === 'claude' ? 'Claude Code' : 'Codex')),
      );
      expect(rows.map((r) => r.key)).toEqual(
        d.entries.map(
          (e) =>
            `${e.agent}:${e.environment.kind}:${e.environment.kind === 'wsl' ? e.environment.distro : ''}`,
        ),
      );
    },
  );

  // 속성: 상태 문구는 상태마다 하나로 정해진다
  test.prop([entryArb])(
    'FR-AGENT-008: 상태를 설치됨, 설치되지 않음, 확인 실패, 멈춤 가운데 하나로 보여 준다',
    (e) => {
      const [row] = toRows(detection(e));
      const expected = {
        installed: '설치됨',
        notInstalled: '설치되지 않음',
        failed: '확인 실패',
        stopped: '멈춤',
      }[e.status.status];

      expect(row?.statusLabel).toBe(expected);
    },
  );

  test.prop([fc.string(), fc.string()])(
    'FR-AGENT-008: 설치됨 줄의 detail은 버전 · 경로다',
    (version, path) => {
      const [row] = toRows(
        detection(entry('claude', macos, { status: 'installed', path, version })),
      );

      expect(row?.detail).toBe(`${version} · ${path}`);
    },
  );
});

describe('panelReducer, canRedetect', () => {
  it('FR-AGENT-008 시나리오 2: 처음 상태는 결과 없이 확인 중이고 다시 감지 버튼이 비활성화된다', () => {
    expect(initialPanelState.result).toBeUndefined();
    expect(initialPanelState.detecting).toBe(true);
    expect(canRedetect(initialPanelState)).toBe(false);
  });

  it('FR-AGENT-008 시나리오 2: 처음 감지가 끝나면 결과로 바뀌고 버튼이 활성화된다', () => {
    const result = detection(entry('claude', macos, { status: 'notInstalled' }));
    const next = panelReducer(initialPanelState, { type: 'detected', result });

    expect(next).toEqual({ result, detecting: false });
    expect(canRedetect(next)).toBe(true);
  });

  it('FR-AGENT-008 시나리오 3: 다시 감지 중에는 이전 결과를 그대로 두고 확인 중이며 버튼이 비활성화된다', () => {
    const previous = detection(entry('codex', windows, { status: 'notInstalled' }));
    const next = panelReducer({ result: previous, detecting: false }, { type: 'redetectStarted' });

    expect(next.result).toBe(previous);
    expect(next.detecting).toBe(true);
    expect(canRedetect(next)).toBe(false);
  });

  it('FR-AGENT-008 시나리오 4: 다시 감지가 끝나면 새 결과로 바뀌고 버튼이 다시 활성화된다', () => {
    const previous = detection(entry('codex', windows, { status: 'notInstalled' }));
    const fresh = detection(
      entry('codex', windows, {
        status: 'installed',
        path: 'C:\\bin\\codex.exe',
        version: '0.146.0',
      }),
    );
    const detecting = panelReducer(
      { result: previous, detecting: false },
      { type: 'redetectStarted' },
    );
    const done = panelReducer(detecting, { type: 'detected', result: fresh });

    expect(done).toEqual({ result: fresh, detecting: false });
    expect(canRedetect(done)).toBe(true);
  });

  it('FR-AGENT-008: 감지 요청이 실패하면 이전 결과를 두고 버튼이 다시 활성화된다', () => {
    const previous = detection(entry('codex', windows, { status: 'notInstalled' }));
    const next = panelReducer({ result: previous, detecting: true }, { type: 'failed' });

    expect(next.result).toBe(previous);
    expect(next.detecting).toBe(false);
    expect(canRedetect(next)).toBe(true);
  });

  test.prop([panelStateArb, detectionArb])(
    'FR-AGENT-008: detected는 어떤 상태에서든 결과를 바꾸고 감지를 끝낸다',
    (state, result) => {
      expect(panelReducer(state, { type: 'detected', result })).toEqual({
        result,
        detecting: false,
      });
    },
  );

  test.prop([panelStateArb])(
    'FR-AGENT-008: redetectStarted는 결과를 그대로 두고 감지 중으로 바꾼다',
    (state) => {
      const next = panelReducer(state, { type: 'redetectStarted' });

      expect(next.result).toBe(state.result);
      expect(next.detecting).toBe(true);
    },
  );

  test.prop([panelStateArb])(
    'FR-AGENT-008: 감지 중일 때만 다시 감지 버튼이 비활성화된다',
    (state) => {
      expect(canRedetect(state)).toBe(!state.detecting);
    },
  );
});
