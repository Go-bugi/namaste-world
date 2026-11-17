import { fc, test } from '@fast-check/vitest';
import { agentEnvironmentKey } from '@renderer/shared/lib/agentLabels';
import type { AgentKind } from '@renderer/types/bindings/AgentKind';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { LoginFailure } from '@renderer/types/bindings/LoginFailure';
import type { LoginMethod } from '@renderer/types/bindings/LoginMethod';
import type { LoginStatus } from '@renderer/types/bindings/LoginStatus';
import { describe, expect, it } from 'vitest';
import {
  canRecheck,
  initialLoginState,
  isLoggedIn,
  loginGuide,
  loginReducer,
  loginStatusLabel,
  type LoginEvent,
  type LoginState,
} from './login';

const macos: Environment = { kind: 'macos' };
const windows: Environment = { kind: 'windows' };
const ubuntu: Environment = { kind: 'wsl', distro: 'Ubuntu' };

const claudeMac = agentEnvironmentKey('claude', macos);
const codexMac = agentEnvironmentKey('codex', macos);

const subscription: LoginStatus = { kind: 'loggedIn', method: 'subscription' };
const loggedOut: LoginStatus = { kind: 'loggedOut' };
const timeout: LoginStatus = { kind: 'failed', reason: { kind: 'timeout' } };

const run = (state: LoginState, ...events: LoginEvent[]): LoginState =>
  events.reduce(loginReducer, state);

const environmentArb = fc.constantFrom<Environment>(macos, windows, ubuntu, {
  kind: 'wsl',
  distro: 'Debian',
});
const agentArb = fc.constantFrom<AgentKind>('claude', 'codex');
const methodArb = fc.constantFrom<LoginMethod>('subscription', 'apiKey', 'cloud', 'other');
const failureArb = fc.constantFrom<LoginFailure>(
  { kind: 'notInstalled' },
  { kind: 'launchFailed' },
  { kind: 'timeout' },
  { kind: 'unreadableOutput' },
);
const statusArb: fc.Arbitrary<LoginStatus> = fc.oneof(
  fc.constant<LoginStatus>({ kind: 'loggedIn' }),
  methodArb.map((method): LoginStatus => ({ kind: 'loggedIn', method })),
  fc.constant(loggedOut),
  failureArb.map((reason): LoginStatus => ({ kind: 'failed', reason })),
);

describe('loginReducer', () => {
  it('FR-START-008 시나리오 1: 확인을 시작하면 그 항목을 확인 중이고, 결과가 오면 그 결과가 보인다', () => {
    const checking = run(initialLoginState, {
      type: 'checkStarted',
      key: claudeMac,
      requestId: 1,
    });
    expect(checking).toEqual({ kind: 'checking', key: claudeMac, requestId: 1 });

    const checked = run(checking, { type: 'checked', requestId: 1, status: subscription });
    expect(checked).toEqual({ kind: 'checked', key: claudeMac, status: subscription });
  });

  it('FR-START-008 시나리오 2: 에이전트를 바꿔 다시 확인하면 앞 항목의 늦은 결과는 버린다', () => {
    const state = run(
      initialLoginState,
      { type: 'checkStarted', key: claudeMac, requestId: 1 },
      { type: 'checkStarted', key: codexMac, requestId: 2 },
      { type: 'checked', requestId: 1, status: subscription },
    );

    expect(state).toEqual({ kind: 'checking', key: codexMac, requestId: 2 });
    expect(isLoggedIn(state, codexMac)).toBe(false);
  });

  it('FR-START-008 시나리오 2: 앞서 확인한 결과가 있어도 다른 항목을 확인하기 시작하면 앞 결과는 보이지 않는다', () => {
    const state = run(
      initialLoginState,
      { type: 'checkStarted', key: claudeMac, requestId: 1 },
      { type: 'checked', requestId: 1, status: subscription },
      { type: 'checkStarted', key: codexMac, requestId: 2 },
    );

    expect(state).toEqual({ kind: 'checking', key: codexMac, requestId: 2 });
    expect(isLoggedIn(state, claudeMac)).toBe(false);
  });

  it('FR-START-008 시나리오 4: 로그인 안 됨에서 다시 확인하면 새 결과로 바뀐다', () => {
    const state = run(
      initialLoginState,
      { type: 'checkStarted', key: claudeMac, requestId: 1 },
      { type: 'checked', requestId: 1, status: loggedOut },
      { type: 'checkStarted', key: claudeMac, requestId: 2 },
      { type: 'checked', requestId: 2, status: subscription },
    );

    expect(state).toEqual({ kind: 'checked', key: claudeMac, status: subscription });
    expect(isLoggedIn(state, claudeMac)).toBe(true);
  });

  it('FR-START-008: 같은 항목이라도 이전 확인의 늦은 결과는 버린다', () => {
    const state = run(
      initialLoginState,
      { type: 'checkStarted', key: claudeMac, requestId: 1 },
      { type: 'checkStarted', key: claudeMac, requestId: 2 },
      { type: 'checked', requestId: 1, status: subscription },
    );

    expect(state).toEqual({ kind: 'checking', key: claudeMac, requestId: 2 });
  });

  it('FR-START-008: 확인을 시작하지 않았으면 결과가 와도 버린다', () => {
    expect(run(initialLoginState, { type: 'checked', requestId: 1, status: subscription })).toEqual(
      initialLoginState,
    );
  });

  test.prop([fc.string(), fc.nat(), fc.nat(), statusArb])(
    'FR-START-008: 지금 확인과 requestId가 다른 결과는 상태를 바꾸지 않는다',
    (key, current, other, status) => {
      fc.pre(current !== other);
      const checking = run(initialLoginState, { type: 'checkStarted', key, requestId: current });

      expect(loginReducer(checking, { type: 'checked', requestId: other, status })).toEqual(
        checking,
      );
    },
  );

  test.prop([fc.string(), fc.nat(), statusArb])(
    'FR-START-008: 지금 확인의 결과는 그 항목의 결과로 담긴다',
    (key, requestId, status) => {
      const state = run(
        initialLoginState,
        { type: 'checkStarted', key, requestId },
        { type: 'checked', requestId, status },
      );

      expect(state).toEqual({ kind: 'checked', key, status });
    },
  );
});

describe('isLoggedIn', () => {
  it('FR-START-005 시나리오 7: 지금 항목의 결과가 로그인됨이면 true다', () => {
    const state: LoginState = { kind: 'checked', key: claudeMac, status: subscription };

    expect(isLoggedIn(state, claudeMac)).toBe(true);
  });

  it('FR-START-005 시나리오 6: 로그인 안 됨, 확인 실패, 확인 중이면 false다', () => {
    expect(isLoggedIn({ kind: 'checked', key: claudeMac, status: loggedOut }, claudeMac)).toBe(
      false,
    );
    expect(isLoggedIn({ kind: 'checked', key: claudeMac, status: timeout }, claudeMac)).toBe(false);
    expect(isLoggedIn({ kind: 'checking', key: claudeMac, requestId: 1 }, claudeMac)).toBe(false);
  });

  it('FR-START-008 시나리오 2: 다른 항목의 로그인됨 결과는 지금 항목의 결과가 아니다', () => {
    const state: LoginState = { kind: 'checked', key: claudeMac, status: subscription };

    expect(isLoggedIn(state, codexMac)).toBe(false);
    expect(isLoggedIn(state, undefined)).toBe(false);
  });

  test.prop([statusArb])('FR-START-005: 로그인됨일 때만 true다', (status) => {
    const state: LoginState = { kind: 'checked', key: claudeMac, status };

    expect(isLoggedIn(state, claudeMac)).toBe(status.kind === 'loggedIn');
  });
});

describe('canRecheck', () => {
  it('FR-START-008: 확인 중에는 다시 확인 버튼이 비활성화된다', () => {
    expect(canRecheck({ kind: 'checking', key: claudeMac, requestId: 1 })).toBe(false);
  });

  it('FR-START-008 시나리오 3: 시간 초과로 확인 실패가 되면 다시 확인 버튼이 활성화된다', () => {
    expect(canRecheck({ kind: 'checked', key: claudeMac, status: timeout })).toBe(true);
  });

  test.prop([statusArb])(
    'FR-START-008: 확인이 끝나면 결과와 상관없이 다시 확인할 수 있다',
    (status) => {
      expect(canRecheck({ kind: 'checked', key: claudeMac, status })).toBe(true);
    },
  );
});

describe('loginGuide', () => {
  it('FR-START-009 시나리오 1: macOS의 Claude Code가 로그인 안 됨이면 claude auth login을 안내한다', () => {
    expect(loginGuide('claude', macos, loggedOut)?.command).toBe('claude auth login');
  });

  it('FR-START-009 시나리오 3: WSL Ubuntu의 Codex가 로그인 안 됨이면 그 배포판 터미널에서 codex login을 안내한다', () => {
    const guide = loginGuide('codex', ubuntu, loggedOut);

    expect(guide?.command).toBe('codex login');
    expect(guide?.where).toContain('WSL');
    expect(guide?.where).toContain('Ubuntu');
  });

  it('FR-START-009: Windows의 Codex가 로그인 안 됨이면 codex login을 안내한다', () => {
    expect(loginGuide('codex', windows, loggedOut)?.command).toBe('codex login');
  });

  it('FR-START-009 시나리오 4: 로그인됨이면 안내가 없다', () => {
    expect(loginGuide('claude', macos, subscription)).toBeUndefined();
  });

  it('FR-START-009: 확인 실패는 로그인 안 됨이 아니므로 안내가 없다', () => {
    expect(loginGuide('codex', ubuntu, timeout)).toBeUndefined();
  });

  test.prop([agentArb, environmentArb, statusArb])(
    'FR-START-009: 로그인 안 됨일 때만 안내가 있다',
    (agent, environment, status) => {
      expect(loginGuide(agent, environment, status) !== undefined).toBe(
        status.kind === 'loggedOut',
      );
    },
  );

  test.prop([agentArb, environmentArb])(
    'FR-START-009: 명령은 실행 환경과 상관없이 에이전트마다 정해져 있다',
    (agent, environment) => {
      expect(loginGuide(agent, environment, loggedOut)?.command).toBe(
        agent === 'claude' ? 'claude auth login' : 'codex login',
      );
    },
  );

  test.prop([agentArb, fc.stringMatching(/^[A-Za-z][A-Za-z0-9-]{0,15}$/)])(
    'FR-START-009: WSL 항목의 안내에는 배포판 이름이 들어간다',
    (agent, distro) => {
      expect(loginGuide(agent, { kind: 'wsl', distro }, loggedOut)?.where).toContain(distro);
    },
  );
});

describe('loginStatusLabel', () => {
  it('FR-START-008: 로그인 안 됨 문구', () => {
    const label = loginStatusLabel(loggedOut);

    expect(label).toContain('로그인 안 됨');
    expect(label).not.toContain('확인 실패');
  });

  it('FR-START-008 시나리오 3: 시간 초과면 확인 실패와 이유가 보인다', () => {
    const label = loginStatusLabel(timeout);

    expect(label).toContain('확인 실패');
    expect(label).toContain('시간 초과');
  });

  it('FR-AGENT-009 시나리오 3: API 키로 로그인됨이면 로그인됨과 방식이 보인다', () => {
    const label = loginStatusLabel({ kind: 'loggedIn', method: 'apiKey' });

    expect(label).toContain('로그인됨');
    expect(label).toContain('API 키');
  });

  it('FR-START-008: 로그인 방식이 다르면 문구도 다르다', () => {
    expect(loginStatusLabel(subscription)).not.toBe(
      loginStatusLabel({ kind: 'loggedIn', method: 'apiKey' }),
    );
  });

  test.prop([failureArb])(
    'FR-AGENT-010: 확인 실패 문구는 로그인 안 됨으로 보이지 않는다',
    (reason) => {
      const label = loginStatusLabel({ kind: 'failed', reason });

      expect(label).toContain('확인 실패');
      expect(label).not.toContain('로그인 안 됨');
      expect(label).not.toContain('로그인됨');
    },
  );

  it('FR-START-008: 확인 실패 이유마다 문구가 다르다', () => {
    const reasons: LoginFailure[] = [
      { kind: 'notInstalled' },
      { kind: 'launchFailed' },
      { kind: 'timeout' },
      { kind: 'unreadableOutput' },
    ];
    const labels = reasons.map((reason) => loginStatusLabel({ kind: 'failed', reason }));

    expect(new Set(labels).size).toBe(reasons.length);
  });

  test.prop([methodArb])(
    'FR-START-008: 로그인됨은 방식과 상관없이 로그인됨으로 보인다',
    (method) => {
      expect(loginStatusLabel({ kind: 'loggedIn', method })).toContain('로그인됨');
    },
  );
});
