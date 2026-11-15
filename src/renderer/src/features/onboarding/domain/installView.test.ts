import { fc, test } from '@fast-check/vitest';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { InstallEntry } from '@renderer/types/bindings/InstallEntry';
import type { InstallFailure } from '@renderer/types/bindings/InstallFailure';
import type { InstallStatus } from '@renderer/types/bindings/InstallStatus';
import { describe, expect, it } from 'vitest';
import { toInstallRows } from './installView';

const macos: Environment = { kind: 'macos' };
const windows: Environment = { kind: 'windows' };
const ubuntu: Environment = { kind: 'wsl', distro: 'Ubuntu' };

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

describe('toInstallRows', () => {
  it('FR-HOOK-015: 설치 결과를 에이전트와 실행 환경마다 상태 문구와 함께 보여 준다', () => {
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

  it('FR-HOOK-015 시나리오 1: Codex 설치가 실패하면 Codex와 그 실행 환경, 실패 이유가 보인다', () => {
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
    'FR-HOOK-015: 에이전트와 실행 환경마다 한 줄씩 entries 순서대로 낸다',
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
  test.prop([entryArb])('FR-HOOK-015 시나리오 1: 실패 줄에만 실패 이유가 붙는다', (entry) => {
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
    'FR-HOOK-015: 에이전트와 실행 환경 조합이 다르면 key가 다르다',
    (entries) => {
      const keys = toInstallRows({ entries }).map((r) => r.key);

      expect(new Set(keys).size).toBe(entries.length);
    },
  );
});
