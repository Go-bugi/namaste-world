import { fc, test } from '@fast-check/vitest';
import { agentEnvironmentKey } from '@renderer/shared/lib/agentLabels';
import type { AgentEntry } from '@renderer/types/bindings/AgentEntry';
import type { AgentKind } from '@renderer/types/bindings/AgentKind';
import type { AgentStatus } from '@renderer/types/bindings/AgentStatus';
import type { Detection } from '@renderer/types/bindings/Detection';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { ModelList } from '@renderer/types/bindings/ModelList';
import type { Selection } from '@renderer/types/bindings/Selection';
import { describe, expect, it } from 'vitest';
import {
  agentChoices,
  canProceed,
  choicesToQuery,
  initialSelectionState,
  selectionReducer,
  toSelection,
  type SelectionEvent,
  type SelectionState,
} from './selection';

const macos: Environment = { kind: 'macos' };
const windows: Environment = { kind: 'windows' };
const ubuntu: Environment = { kind: 'wsl', distro: 'Ubuntu' };

const installed: AgentStatus = { status: 'installed', path: '/bin/x', version: '1.0.0' };
const notInstalled: AgentStatus = { status: 'notInstalled' };

const entry = (agent: AgentKind, environment: Environment, status: AgentStatus): AgentEntry => ({
  agent,
  environment,
  status,
});

const detection = (...entries: AgentEntry[]): Detection => ({ entries });

const claudeMac = agentEnvironmentKey('claude', macos);
const codexMac = agentEnvironmentKey('codex', macos);
const codexWindows = agentEnvironmentKey('codex', windows);
const codexUbuntu = agentEnvironmentKey('codex', ubuntu);

const claudeList: ModelList = {
  options: [
    { value: 'default', label: 'Default', detail: 'Opus 5.5' },
    { value: 'opus', label: 'Opus' },
    { value: 'sonnet', label: 'Sonnet' },
    { value: 'fable', label: 'Fable' },
    { value: 'haiku', label: 'Haiku' },
  ],
  defaultValue: 'default',
  source: 'queried',
};

const codexList: ModelList = {
  options: [
    { value: 'gpt-5.6-sol', label: 'GPT-5.6-Sol' },
    { value: 'gpt-5.6-terra', label: 'GPT-5.6-Terra' },
    { value: 'gpt-5.6-luna', label: 'GPT-5.6-Luna' },
  ],
  defaultValue: 'gpt-5.6-terra',
  source: 'queried',
};

const run = (state: SelectionState, ...events: SelectionEvent[]): SelectionState =>
  events.reduce(selectionReducer, state);

/** macOS의 Claude Code와 Codex가 모두 설치된 감지 결과 */
const macBoth = detection(entry('claude', macos, installed), entry('codex', macos, installed));

describe('agentChoices', () => {
  it('FR-START-003 시나리오 1: 설치됨인 항목만 entries 순서대로 선택지가 된다', () => {
    const choices = agentChoices(
      detection(
        entry('claude', windows, notInstalled),
        entry('codex', windows, installed),
        entry('claude', ubuntu, notInstalled),
        entry('codex', ubuntu, installed),
      ),
    );

    expect(choices).toEqual([
      { key: codexWindows, agent: 'codex', environment: windows, label: 'Codex · Windows' },
      { key: codexUbuntu, agent: 'codex', environment: ubuntu, label: 'Codex · WSL · Ubuntu' },
    ]);
  });

  it('FR-START-003: 확인 실패와 멈춤 항목은 선택지에 없다', () => {
    const choices = agentChoices(
      detection(
        entry('claude', macos, installed),
        entry('codex', ubuntu, { status: 'failed', reason: { kind: 'timeout' } }),
        entry('claude', { kind: 'wsl', distro: 'Debian' }, { status: 'stopped' }),
      ),
    );

    expect(choices.map((c) => c.label)).toEqual(['Claude Code · macOS']);
  });

  const statusArb: fc.Arbitrary<AgentStatus> = fc.oneof(
    fc.constant(installed),
    fc.constant(notInstalled),
    fc.constant<AgentStatus>({ status: 'stopped' }),
    fc.constant<AgentStatus>({ status: 'failed', reason: { kind: 'versionUnreadable' } }),
  );
  const entryArb: fc.Arbitrary<AgentEntry> = fc.record({
    agent: fc.constantFrom<AgentKind>('claude', 'codex'),
    environment: fc.constantFrom<Environment>(macos, windows, ubuntu),
    status: statusArb,
  });

  test.prop([fc.array(entryArb, { maxLength: 8 })])(
    'FR-START-003: 선택지는 설치됨 항목과 같은 수, 같은 순서다',
    (entries) => {
      const choices = agentChoices({ entries });
      const expected = entries
        .filter((e) => e.status.status === 'installed')
        .map((e) => agentEnvironmentKey(e.agent, e.environment));

      expect(choices.map((c) => c.key)).toEqual(expected);
    },
  );
});

describe('choicesToQuery', () => {
  const both = (): ReturnType<typeof agentChoices> =>
    agentChoices(detection(entry('codex', windows, installed), entry('codex', ubuntu, installed)));

  it('FR-START-007 시나리오 3: 감지가 끝나면 Windows의 Codex만 바로 조회하고 WSL 항목은 조회하지 않는다', () => {
    expect(choicesToQuery(both(), undefined).map((c) => c.key)).toEqual([codexWindows]);
  });

  it('FR-START-007 시나리오 3: Windows 항목을 골라 둔 상태여도 WSL 항목은 바로 조회하지 않는다', () => {
    expect(choicesToQuery(both(), codexWindows).map((c) => c.key)).toEqual([codexWindows]);
  });

  it('FR-START-007 시나리오 4: 이미 골라진 WSL 항목은 함께 조회한다', () => {
    const keys = choicesToQuery(both(), codexUbuntu).map((c) => c.key);

    expect(keys).toHaveLength(2);
    expect(new Set(keys)).toEqual(new Set([codexWindows, codexUbuntu]));
  });

  it('FR-START-007: macOS 항목은 모두 바로 조회한다', () => {
    const keys = choicesToQuery(agentChoices(macBoth), undefined).map((c) => c.key);

    expect(new Set(keys)).toEqual(new Set([claudeMac, codexMac]));
    expect(keys).toHaveLength(2);
  });
});

describe('selectionReducer', () => {
  it('FR-START-003 시나리오 2: 설치된 항목이 macOS의 Claude Code 하나뿐이면 미리 골라진다', () => {
    const state = run(initialSelectionState, {
      type: 'detected',
      detection: detection(entry('claude', macos, installed), entry('codex', macos, notInstalled)),
    });

    expect(state.selectedKey).toBe(claudeMac);
  });

  it('FR-START-003: 설치된 항목이 여럿이고 저장한 선택이 없으면 아무것도 미리 고르지 않는다', () => {
    const state = run(
      initialSelectionState,
      { type: 'savedLoaded', saved: undefined },
      { type: 'detected', detection: macBoth },
    );

    expect(state.choices).toHaveLength(2);
    expect(state.selectedKey).toBeUndefined();
  });

  it('FR-START-003 시나리오 3: 설치된 에이전트가 없으면 선택지가 비고 다음 버튼은 비활성화된다', () => {
    const state = run(initialSelectionState, {
      type: 'detected',
      detection: detection(
        entry('claude', macos, notInstalled),
        entry('codex', macos, notInstalled),
      ),
    });

    expect(state.choices).toEqual([]);
    expect(state.selectedKey).toBeUndefined();
    expect(canProceed(state, false)).toBe(false);
  });

  it('FR-START-007: modelsLoading이면 그 항목의 목록이 불러오는 중이 된다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoading', key: claudeMac },
    );

    expect(state.models[claudeMac]).toEqual({ status: 'loading' });
    expect(state.models[codexMac]).toBeUndefined();
  });

  it('FR-START-004: modelsLoaded면 그 항목의 목록이 준비된다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoading', key: claudeMac },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
    );

    expect(state.models[claudeMac]).toEqual({ status: 'ready', list: claudeList });
  });

  it('FR-START-004 시나리오 3: Claude Code를 고르면 default 모델이 골라진다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
    );

    expect(state.selectedKey).toBe(claudeMac);
    expect(state.model).toBe('default');
  });

  it('FR-START-004 시나리오 3: 목록이 나중에 도착해도 골라 둔 Claude Code에 default가 골라진다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelsLoading', key: claudeMac },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
    );

    expect(state.model).toBe('default');
  });

  it('FR-START-004 시나리오 1: Claude Code에서 sonnet을 고른 뒤 Codex로 바꾸면 Codex의 기본 모델이 골라진다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'modelsLoaded', key: codexMac, list: codexList },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelPicked', value: 'sonnet' },
      { type: 'agentPicked', key: codexMac },
    );

    expect(state.selectedKey).toBe(codexMac);
    expect(state.model).toBe('gpt-5.6-terra');
  });

  it('FR-START-004: 목록이 아직 없는 에이전트로 바꾸면 고른 모델이 비고 다음 버튼은 비활성화된다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelPicked', value: 'sonnet' },
      { type: 'agentPicked', key: codexMac },
    );

    expect(state.selectedKey).toBe(codexMac);
    expect(state.model).toBeUndefined();
    expect(canProceed(state, false)).toBe(false);
  });

  it('FR-START-004: 같은 에이전트를 다시 고르면 고른 모델이 그대로다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelPicked', value: 'sonnet' },
      { type: 'agentPicked', key: claudeMac },
    );

    expect(state.model).toBe('sonnet');
  });

  it('FR-START-004: 고르지 않은 항목의 목록이 도착해도 고른 모델은 그대로다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelPicked', value: 'sonnet' },
      { type: 'modelsLoaded', key: codexMac, list: codexList },
    );

    expect(state.selectedKey).toBe(claudeMac);
    expect(state.model).toBe('sonnet');
    expect(state.models[codexMac]).toEqual({ status: 'ready', list: codexList });
  });

  /** macOS의 Claude Code와 sonnet을 고른 상태 */
  const claudeSonnet = (): SelectionState =>
    run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelPicked', value: 'sonnet' },
    );

  it('FR-START-004 시나리오 2: 다시 감지 뒤에도 Claude Code가 설치됨이고 새 목록에 sonnet이 있으면 그대로 골라져 있다', () => {
    const afterDetect = run(claudeSonnet(), { type: 'detected', detection: macBoth });

    expect(afterDetect.selectedKey).toBe(claudeMac);
    expect(afterDetect.model).toBe('sonnet');

    const afterQuery = run(
      afterDetect,
      { type: 'modelsLoading', key: claudeMac },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
    );

    expect(afterQuery.selectedKey).toBe(claudeMac);
    expect(afterQuery.model).toBe('sonnet');
  });

  it('FR-START-004: 다시 조회한 목록에 고른 모델이 없으면 그 목록의 기본 모델로 다시 고른다', () => {
    const withoutSonnet: ModelList = {
      ...claudeList,
      options: claudeList.options.filter((o) => o.value !== 'sonnet'),
    };

    const state = run(
      claudeSonnet(),
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoading', key: claudeMac },
      { type: 'modelsLoaded', key: claudeMac, list: withoutSonnet },
    );

    expect(state.model).toBe('default');
  });

  it('FR-START-007 시나리오 4: 골라 둔 WSL 항목을 다시 조회하는 동안 모델 목록은 불러오는 중이다', () => {
    const wsl = detection(entry('codex', windows, installed), entry('codex', ubuntu, installed));

    const state = run(
      initialSelectionState,
      { type: 'detected', detection: wsl },
      { type: 'agentPicked', key: codexUbuntu },
      { type: 'modelsLoading', key: codexUbuntu },
      { type: 'modelsLoaded', key: codexUbuntu, list: codexList },
      { type: 'detected', detection: wsl },
      { type: 'modelsLoading', key: codexUbuntu },
    );

    expect(state.selectedKey).toBe(codexUbuntu);
    expect(state.models[codexUbuntu]).toEqual({ status: 'loading' });
  });

  it('FR-START-004: 다시 감지에서 고른 에이전트가 사라지고 남은 항목이 하나면 그 항목으로 다시 고른다', () => {
    const state = run(claudeSonnet(), {
      type: 'detected',
      detection: detection(entry('claude', macos, notInstalled), entry('codex', macos, installed)),
    });

    expect(state.selectedKey).toBe(codexMac);
  });

  it('FR-START-004: 다시 감지에서 고른 에이전트가 사라지고 남은 항목이 여럿이면 다시 고르지 않는다', () => {
    const state = run(claudeSonnet(), {
      type: 'detected',
      detection: detection(
        entry('claude', macos, notInstalled),
        entry('codex', windows, installed),
        entry('codex', ubuntu, installed),
      ),
    });

    expect(state.selectedKey).toBeUndefined();
    expect(toSelection(state)).toBeUndefined();
    expect(canProceed(state, false)).toBe(false);
  });

  const savedCodex: Selection = { agent: 'codex', environment: macos, model: 'gpt-5.6-sol' };

  it('FR-START-006 시나리오 2: 지난 실행의 Codex와 gpt-5.6-sol이 모두 선택지에 있으면 미리 골라진다', () => {
    const state = run(
      initialSelectionState,
      { type: 'savedLoaded', saved: savedCodex },
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoading', key: codexMac },
      { type: 'modelsLoaded', key: codexMac, list: codexList },
    );

    expect(state.selectedKey).toBe(codexMac);
    expect(state.model).toBe('gpt-5.6-sol');
    expect(toSelection(state)).toEqual(savedCodex);
  });

  it('FR-START-006: 저장한 모델이 목록에 없으면 기본 모델을 고른다', () => {
    const state = run(
      initialSelectionState,
      { type: 'savedLoaded', saved: { ...savedCodex, model: 'gpt-4' } },
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: codexMac, list: codexList },
    );

    expect(state.selectedKey).toBe(codexMac);
    expect(state.model).toBe('gpt-5.6-terra');
  });

  it('FR-START-006 시나리오 3: 저장한 Codex가 설치되지 않음으로 나오면 고르지 않고 FR-START-003대로 고른다', () => {
    const state = run(
      initialSelectionState,
      { type: 'savedLoaded', saved: savedCodex },
      {
        type: 'detected',
        detection: detection(
          entry('claude', macos, installed),
          entry('codex', macos, notInstalled),
        ),
      },
    );

    expect(state.selectedKey).toBe(claudeMac);
  });

  it('FR-START-006 시나리오 3: 저장한 Codex가 사라지고 남은 항목이 여럿이면 아무것도 고르지 않는다', () => {
    const state = run(
      initialSelectionState,
      { type: 'savedLoaded', saved: savedCodex },
      {
        type: 'detected',
        detection: detection(
          entry('claude', macos, installed),
          entry('claude', ubuntu, installed),
          entry('codex', macos, notInstalled),
        ),
      },
    );

    expect(state.selectedKey).toBeUndefined();
  });

  it('FR-START-006: 저장한 실행 환경이 다르면 같은 에이전트라도 미리 고르지 않는다', () => {
    const state = run(
      initialSelectionState,
      { type: 'savedLoaded', saved: { ...savedCodex, environment: ubuntu } },
      {
        type: 'detected',
        detection: detection(
          entry('codex', windows, installed),
          entry('claude', windows, installed),
        ),
      },
    );

    expect(state.selectedKey).toBeUndefined();
  });
});

describe('canProceed', () => {
  const ready = (): SelectionState =>
    run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
    );

  it('FR-START-005 시나리오 1: 에이전트와 모델이 모두 골라졌고 감지 중이 아니면 다음 버튼이 활성화된다', () => {
    expect(canProceed(ready(), false)).toBe(true);
  });

  it('FR-START-005: 감지나 다시 감지가 진행 중이면 다음 버튼은 비활성화된다', () => {
    expect(canProceed(ready(), true)).toBe(false);
  });

  it('FR-START-005 시나리오 2: 에이전트는 골랐지만 모델이 골라지지 않으면 다음 버튼은 비활성화된다', () => {
    const noModel = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'agentPicked', key: codexMac },
    );

    expect(noModel.selectedKey).toBe(codexMac);
    expect(canProceed(noModel, false)).toBe(false);
  });

  it('FR-START-005: 에이전트를 고르지 않았으면 다음 버튼은 비활성화된다', () => {
    const state = run(initialSelectionState, { type: 'detected', detection: macBoth });

    expect(canProceed(state, false)).toBe(false);
  });

  const eventArb: fc.Arbitrary<SelectionEvent> = fc.oneof(
    fc.constant<SelectionEvent>({ type: 'detected', detection: macBoth }),
    fc
      .constantFrom(claudeMac, codexMac)
      .map((key): SelectionEvent => ({ type: 'agentPicked', key })),
    fc
      .constantFrom(claudeMac, codexMac)
      .map((key): SelectionEvent => ({ type: 'modelsLoading', key })),
    fc
      .constantFrom<[string, ModelList]>([claudeMac, claudeList], [codexMac, codexList])
      .map(([key, list]): SelectionEvent => ({ type: 'modelsLoaded', key, list })),
    fc
      .constantFrom('sonnet', 'gpt-5.6-sol')
      .map((value): SelectionEvent => ({ type: 'modelPicked', value })),
  );

  test.prop([fc.array(eventArb, { maxLength: 12 })])(
    'FR-START-005: 감지 중이면 어떤 상태에서도 다음 버튼은 비활성화된다',
    (events) => {
      expect(canProceed(run(initialSelectionState, ...events), true)).toBe(false);
    },
  );

  test.prop([fc.array(eventArb, { maxLength: 12 })])(
    'FR-START-005: 다음 버튼이 활성화되면 넘길 선택 값이 있다',
    (events) => {
      const state = run(initialSelectionState, ...events);

      expect(canProceed(state, false)).toBe(toSelection(state) !== undefined);
    },
  );
});

describe('toSelection', () => {
  it('FR-START-006 시나리오 1: macOS의 Claude Code와 모델을 골랐으면 그 값을 넘긴다', () => {
    const state = run(
      initialSelectionState,
      { type: 'detected', detection: macBoth },
      { type: 'modelsLoaded', key: claudeMac, list: claudeList },
      { type: 'agentPicked', key: claudeMac },
      { type: 'modelPicked', value: 'opus' },
    );

    expect(toSelection(state)).toEqual({ agent: 'claude', environment: macos, model: 'opus' });
  });

  it('FR-START-006: 고르지 않았으면 넘길 값이 없다', () => {
    expect(toSelection(initialSelectionState)).toBeUndefined();
  });
});
