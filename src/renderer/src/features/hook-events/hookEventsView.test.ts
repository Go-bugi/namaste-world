import { fc, test } from '@fast-check/vitest';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { HookEvent } from '@renderer/types/bindings/HookEvent';
import type { InstallEntry } from '@renderer/types/bindings/InstallEntry';
import type { InstallFailure } from '@renderer/types/bindings/InstallFailure';
import type { InstallStatus } from '@renderer/types/bindings/InstallStatus';
import { describe, expect, it } from 'vitest';
import { appendEvent, toEventRow, toInstallRows } from './hookEventsView';

const macos: Environment = { kind: 'macos' };
const windows: Environment = { kind: 'windows' };
const ubuntu: Environment = { kind: 'wsl', distro: 'Ubuntu' };

const hookEvent = (n: number, overrides: Partial<HookEvent> = {}): HookEvent => ({
  agent: 'claude',
  event: `Event${n}`,
  receivedAt: 1_700_000_000_000 + n,
  truncated: false,
  ...overrides,
});

const pad2 = (n: number): string => String(n).padStart(2, '0');

const eventArb: fc.Arbitrary<HookEvent> = fc.record({
  agent: fc.constantFrom<HookEvent['agent']>('claude', 'codex'),
  event: fc.string({ minLength: 1 }),
  receivedAt: fc.integer({ min: 0, max: 4_102_444_800_000 }),
  truncated: fc.boolean(),
});

const environmentArb: fc.Arbitrary<Environment> = fc.oneof(
  fc.constant<Environment>(macos),
  fc.constant<Environment>(windows),
  fc.string({ minLength: 1 }).map((distro): Environment => ({ kind: 'wsl', distro })),
);

const failureArb: fc.Arbitrary<InstallFailure> = fc.oneof(
  fc.string({ minLength: 1 }).map((file): InstallFailure => ({ kind: 'invalidConfig', file })),
  fc.string({ minLength: 1 }).map((file): InstallFailure => ({ kind: 'notWritable', file })),
  fc.string({ minLength: 1 }).map((file): InstallFailure => ({ kind: 'codexHooksDisabled', file })),
  fc.string({ minLength: 1 }).map((message): InstallFailure => ({ kind: 'io', message })),
);

const statusArb: fc.Arbitrary<InstallStatus> = fc.oneof(
  fc.constant<InstallStatus>({ status: 'installed' }),
  fc.constant<InstallStatus>({ status: 'alreadyInstalled' }),
  failureArb.map((reason): InstallStatus => ({ status: 'failed', reason })),
);

const entryArb: fc.Arbitrary<InstallEntry> = fc.record({
  agent: fc.constantFrom<InstallEntry['agent']>('claude', 'codex'),
  environment: environmentArb,
  status: statusArb,
});

const comboId = (e: InstallEntry): string =>
  JSON.stringify([
    e.agent,
    e.environment.kind,
    e.environment.kind === 'wsl' ? e.environment.distro : '',
  ]);

describe('appendEvent', () => {
  it('FR-HOOK-010 시나리오 1: 받은 이벤트를 받은 순서대로 끝에 붙인다', () => {
    const names = ['UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'Stop'];
    const list = names.reduce<readonly HookEvent[]>(
      (acc, event, i) => appendEvent(acc, hookEvent(i, { event })),
      [],
    );

    expect(list.map((e) => e.event)).toEqual(names);
  });

  it('FR-HOOK-010 시나리오 2: 201개를 받으면 두 번째부터 마지막까지 200개가 남는다', () => {
    const received = Array.from({ length: 201 }, (_, i) => hookEvent(i));
    const list = received.reduce<readonly HookEvent[]>((acc, e) => appendEvent(acc, e), []);

    // 첫 번째(0번)만 빠지고 1번부터 200번까지 남는다
    expect(list).toHaveLength(200);
    expect(list).toEqual(received.slice(1));
  });

  it('FR-HOOK-010: 입력 배열은 바꾸지 않는다', () => {
    const list = Object.freeze([hookEvent(0), hookEvent(1)]);
    const snapshot = [...list];

    appendEvent(list, hookEvent(2), 2);

    expect(list).toEqual(snapshot);
  });

  // 속성: 결과는 "기존 목록 + 새 이벤트"의 뒤쪽 limit개다
  test.prop([fc.array(eventArb, { maxLength: 30 }), eventArb, fc.integer({ min: 1, max: 20 })])(
    'FR-HOOK-010: 결과는 limit개를 넘지 않고, 오래된 것부터 빠지며, 새 이벤트가 맨 끝이다',
    (list, event, limit) => {
      const before = [...list];
      const result = appendEvent(list, event, limit);
      const all = [...list, event];

      expect(result.length).toBe(Math.min(all.length, limit));
      expect(result).toEqual(all.slice(all.length - result.length));
      expect(result[result.length - 1]).toBe(event);
      expect(list).toEqual(before);
    },
  );
});

describe('toEventRow', () => {
  it('FR-HOOK-010 시나리오 1: Claude Code 이벤트 줄에 받은 시각, 에이전트, 이벤트 이름을 보여 준다', () => {
    const receivedAt = new Date(2026, 10, 14, 9, 5, 7).getTime();
    const row = toEventRow(
      hookEvent(0, { agent: 'claude', event: 'UserPromptSubmit', receivedAt, sessionId: 's1' }),
      0,
    );

    expect(row.time).toBe('09:05:07');
    expect(row.agentLabel).toBe('Claude Code');
    expect(row.event).toBe('UserPromptSubmit');
  });

  it('FR-HOOK-010: Codex 이벤트 줄의 에이전트는 Codex다', () => {
    const row = toEventRow(hookEvent(0, { agent: 'codex', event: 'Stop' }), 3);

    expect(row.agentLabel).toBe('Codex');
    expect(row.event).toBe('Stop');
  });

  // 속성: 시각은 로컬 시각의 시, 분, 초를 두 자리씩 맞춘 HH:MM:SS다
  test.prop([eventArb, fc.nat({ max: 199 })])(
    'FR-HOOK-010: 받은 시각을 로컬 HH:MM:SS로 보여 준다',
    (event, index) => {
      const d = new Date(event.receivedAt);
      const expected = `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;

      expect(toEventRow(event, index).time).toBe(expected);
    },
  );

  // 속성: 한 목록 안의 줄은 key가 겹치지 않는다
  test.prop([fc.array(eventArb, { minLength: 1, maxLength: 50 })])(
    'FR-HOOK-010: 목록 안 위치가 다른 줄은 key가 다르다',
    (events) => {
      const keys = events.map((e, i) => toEventRow(e, i).key);

      expect(new Set(keys).size).toBe(events.length);
    },
  );

  it('FR-HOOK-010: 같은 이벤트라도 목록 안 위치가 다르면 key가 다르다', () => {
    const e = hookEvent(0);

    expect(toEventRow(e, 0).key).not.toBe(toEventRow(e, 1).key);
  });
});

describe('toInstallRows', () => {
  it('FR-HOOK-010: 설치 결과를 에이전트와 실행 환경마다 상태 문구와 함께 보여 준다', () => {
    const rows = toInstallRows({
      entries: [
        { agent: 'claude', environment: macos, status: { status: 'installed' } },
        { agent: 'codex', environment: windows, status: { status: 'alreadyInstalled' } },
        { agent: 'codex', environment: ubuntu, status: { status: 'installed' } },
      ],
    });

    expect(rows.map((r) => [r.agentLabel, r.environmentLabel, r.statusLabel])).toEqual([
      ['Claude Code', 'macOS', '새로 설치함'],
      ['Codex', 'Windows', '이미 설치됨'],
      ['Codex', 'WSL · Ubuntu', '새로 설치함'],
    ]);
    expect(rows.map((r) => r.detail)).toEqual([undefined, undefined, undefined]);
  });

  it('FR-HOOK-010 시나리오 3: Codex 설치가 실패하면 Codex와 그 실행 환경, 실패 이유가 보인다', () => {
    const [row] = toInstallRows({
      entries: [
        {
          agent: 'codex',
          environment: ubuntu,
          status: {
            status: 'failed',
            reason: { kind: 'notWritable', file: '/home/u/.codex/hooks.json' },
          },
        },
      ],
    });

    expect(row?.agentLabel).toBe('Codex');
    expect(row?.environmentLabel).toBe('WSL · Ubuntu');
    expect(row?.statusLabel).toBe('실패');
    expect(row?.detail).toEqual(expect.any(String));
    expect(row?.detail?.length).toBeGreaterThan(0);
  });

  it('FR-HOOK-007 시나리오 1: Claude Code는 새로 설치함, Codex는 실패와 이유가 함께 나온다', () => {
    const rows = toInstallRows({
      entries: [
        { agent: 'claude', environment: macos, status: { status: 'installed' } },
        {
          agent: 'codex',
          environment: macos,
          status: {
            status: 'failed',
            reason: { kind: 'notWritable', file: '/Users/u/.codex/hooks.json' },
          },
        },
      ],
    });

    expect(rows.map((r) => [r.agentLabel, r.statusLabel])).toEqual([
      ['Claude Code', '새로 설치함'],
      ['Codex', '실패'],
    ]);
    expect(rows[0]?.detail).toBeUndefined();
    expect(rows[1]?.detail?.length).toBeGreaterThan(0);
  });

  it('FR-HOOK-011 시나리오 2: 같은 Codex가 Windows와 WSL에 있으면 환경마다 따로 한 줄씩 나온다', () => {
    const rows = toInstallRows({
      entries: [
        { agent: 'codex', environment: windows, status: { status: 'installed' } },
        { agent: 'codex', environment: ubuntu, status: { status: 'installed' } },
      ],
    });

    expect(rows.map((r) => r.environmentLabel)).toEqual(['Windows', 'WSL · Ubuntu']);
    expect(rows[0]?.key).not.toBe(rows[1]?.key);
  });

  // 속성: 줄은 entries와 한 대 한 대응이고 순서, 에이전트, 환경, 상태 문구가 entries에서 정해진다
  test.prop([fc.array(entryArb, { maxLength: 8 })])(
    'FR-HOOK-010: 에이전트와 실행 환경마다 한 줄씩 entries 순서대로 낸다',
    (entries) => {
      const rows = toInstallRows({ entries });

      expect(rows).toHaveLength(entries.length);
      expect(rows.map((r) => r.agentLabel)).toEqual(
        entries.map((e) => (e.agent === 'claude' ? 'Claude Code' : 'Codex')),
      );
      expect(rows.map((r) => r.environmentLabel)).toEqual(
        entries.map((e) =>
          e.environment.kind === 'macos'
            ? 'macOS'
            : e.environment.kind === 'windows'
              ? 'Windows'
              : `WSL · ${e.environment.distro}`,
        ),
      );
      expect(rows.map((r) => r.statusLabel)).toEqual(
        entries.map((e) =>
          e.status.status === 'installed'
            ? '새로 설치함'
            : e.status.status === 'alreadyInstalled'
              ? '이미 설치됨'
              : '실패',
        ),
      );
    },
  );

  // 속성: 실패한 줄에만 비어 있지 않은 이유가 붙는다
  test.prop([entryArb])('FR-HOOK-010 시나리오 3: 실패 줄에만 실패 이유가 붙는다', (entry) => {
    const [row] = toInstallRows({ entries: [entry] });

    if (entry.status.status === 'failed') {
      expect(typeof row?.detail).toBe('string');
      expect(row?.detail?.length).toBeGreaterThan(0);
    } else {
      expect(row?.detail).toBeUndefined();
    }
  });

  // 속성: 에이전트와 실행 환경이 다른 줄은 key가 겹치지 않는다
  test.prop([fc.uniqueArray(entryArb, { selector: comboId, maxLength: 8 })])(
    'FR-HOOK-010: 에이전트와 실행 환경 조합이 다르면 key가 다르다',
    (entries) => {
      const keys = toInstallRows({ entries }).map((r) => r.key);

      expect(new Set(keys).size).toBe(entries.length);
    },
  );
});
