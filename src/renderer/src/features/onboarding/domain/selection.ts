import type { AgentKind } from '@renderer/types/bindings/AgentKind';
import type { Detection } from '@renderer/types/bindings/Detection';
import type { Environment } from '@renderer/types/bindings/Environment';
import type { ModelList } from '@renderer/types/bindings/ModelList';
import type { Selection } from '@renderer/types/bindings/Selection';
import {
  agentEnvironmentKey,
  agentLabel,
  environmentLabel,
} from '@renderer/shared/lib/agentLabels';

/** 에이전트 선택지 하나. key는 `agentEnvironmentKey`, label은 `${에이전트 이름} · ${실행 환경 이름}`이다. */
export type AgentChoice = {
  readonly key: string;
  readonly agent: AgentKind;
  readonly environment: Environment;
  readonly label: string;
};

/** 감지 결과에서 설치됨인 항목만 entries 순서대로 선택지로 만든다. */
export function agentChoices(detection: Detection): readonly AgentChoice[] {
  return detection.entries
    .filter((entry) => entry.status.status === 'installed')
    .map(({ agent, environment }) => ({
      key: agentEnvironmentKey(agent, environment),
      agent,
      environment,
      label: `${agentLabel(agent)} · ${environmentLabel(environment)}`,
    }));
}

/** 감지가 끝났을 때 바로 조회할 항목(WSL이 아닌 것 + 이미 골라진 WSL 항목). FR-START-007 */
export function choicesToQuery(
  choices: readonly AgentChoice[],
  selectedKey: string | undefined,
): readonly AgentChoice[] {
  return choices.filter(
    (choice) => choice.environment.kind !== 'wsl' || choice.key === selectedKey,
  );
}

/** 항목 하나의 모델 목록 상태. */
export type ModelsEntry =
  { readonly status: 'loading' } | { readonly status: 'ready'; readonly list: ModelList };

/** 항목별 모델 목록 상태. 키가 없으면 아직 조회하지 않았다. */
export type ModelsState = { readonly [choiceKey: string]: ModelsEntry };

/** 선택 단계의 상태. */
export type SelectionState = {
  readonly choices: readonly AgentChoice[];
  readonly selectedKey?: string;
  readonly model?: string;
  readonly models: ModelsState;
  readonly saved?: Selection;
};

export type SelectionEvent =
  | { readonly type: 'detected'; readonly detection: Detection }
  | { readonly type: 'savedLoaded'; readonly saved: Selection | undefined }
  | { readonly type: 'agentPicked'; readonly key: string }
  | { readonly type: 'modelsLoading'; readonly key: string }
  | { readonly type: 'modelsLoaded'; readonly key: string; readonly list: ModelList }
  | { readonly type: 'modelPicked'; readonly value: string };

/** 감지 결과를 받기 전의 상태. */
export const initialSelectionState: SelectionState = { choices: [], models: {} };

/**
 * 선택 상태 전이.
 * - savedLoaded는 항상 첫 detected보다 먼저 온다(앱이 저장값을 먼저 읽는다). 역순은 다루지 않는다.
 * - detected: 선택지를 새로 만들고, 골라진 항목이 여전히 있으면 유지, 없으면 FR-START-003대로(하나면 그것, saved가 있으면 그것) 고른다.
 * - modelsLoaded: 그 항목이 골라져 있고 고른 모델이 새 목록에 없으면 saved 모델(목록에 있을 때) → defaultValue 순으로 고른다.
 * - agentPicked: 다른 항목이면 모델을 비우고, 목록이 ready면 defaultValue를 고른다.
 */
export function selectionReducer(state: SelectionState, event: SelectionEvent): SelectionState {
  switch (event.type) {
    case 'savedLoaded':
      return event.saved === undefined ? omit(state, 'saved') : { ...state, saved: event.saved };
    case 'detected':
      return onDetected(state, agentChoices(event.detection));
    case 'agentPicked':
      if (event.key === state.selectedKey) return state;
      return withModel({ ...state, selectedKey: event.key }, defaultModelOf(state, event.key));
    case 'modelsLoading':
      return { ...state, models: { ...state.models, [event.key]: { status: 'loading' } } };
    case 'modelsLoaded': {
      const next: SelectionState = {
        ...state,
        models: { ...state.models, [event.key]: { status: 'ready', list: event.list } },
      };
      if (event.key !== state.selectedKey) return next;
      if (state.model !== undefined && hasModel(event.list, state.model)) return next;
      return withModel(next, preferredModel(next, event.key, event.list));
    }
    case 'modelPicked':
      return state.selectedKey === undefined ? state : { ...state, model: event.value };
  }
}

/** 새 선택지로 바꾼다. 골라 둔 항목이 남아 있으면 그대로 두고, 없으면 저장값 → 하나뿐인 항목 순으로 고른다. */
function onDetected(state: SelectionState, choices: readonly AgentChoice[]): SelectionState {
  const next = { ...state, choices };
  if (choices.some((choice) => choice.key === state.selectedKey)) return next;
  const savedKey =
    state.saved === undefined
      ? undefined
      : agentEnvironmentKey(state.saved.agent, state.saved.environment);
  const picked =
    choices.find((choice) => choice.key === savedKey) ??
    (choices.length === 1 ? choices[0] : undefined);
  if (picked === undefined) return omit(omit(next, 'selectedKey'), 'model');
  const ready = readyList(next, picked.key);
  return withModel(
    { ...next, selectedKey: picked.key },
    ready === undefined ? undefined : preferredModel(next, picked.key, ready),
  );
}

/** 항목의 목록이 준비됐으면 그 목록. */
function readyList(state: SelectionState, key: string): ModelList | undefined {
  const entry = state.models[key];
  return entry?.status === 'ready' ? entry.list : undefined;
}

function hasModel(list: ModelList, value: string): boolean {
  return list.options.some((option) => option.value === value);
}

/** 사용자가 에이전트를 바꿨을 때의 모델. 목록이 있으면 기본 모델, 없으면 고르지 않는다. */
function defaultModelOf(state: SelectionState, key: string): string | undefined {
  return readyList(state, key)?.defaultValue;
}

/** 저장한 선택이 이 항목이고 그 모델이 목록에 있으면 그 모델, 아니면 목록의 기본 모델. */
function preferredModel(state: SelectionState, key: string, list: ModelList): string {
  const saved = state.saved;
  const savedModel =
    saved !== undefined && agentEnvironmentKey(saved.agent, saved.environment) === key
      ? saved.model
      : undefined;
  return savedModel !== undefined && hasModel(list, savedModel) ? savedModel : list.defaultValue;
}

function withModel(state: SelectionState, model: string | undefined): SelectionState {
  return model === undefined ? omit(state, 'model') : { ...state, model };
}

/** 선택 필드를 지운 새 상태. exactOptionalPropertyTypes에서 undefined를 넣지 않기 위해 키를 뺀다. */
function omit(state: SelectionState, key: 'saved' | 'selectedKey' | 'model'): SelectionState {
  const { saved, selectedKey, model, ...rest } = state;
  const kept = { saved, selectedKey, model };
  return {
    ...rest,
    ...(key !== 'saved' && kept.saved !== undefined ? { saved: kept.saved } : {}),
    ...(key !== 'selectedKey' && kept.selectedKey !== undefined
      ? { selectedKey: kept.selectedKey }
      : {}),
    ...(key !== 'model' && kept.model !== undefined ? { model: kept.model } : {}),
  };
}

/** 모델 목록 다시 불러오기 버튼을 누를 수 있는가. 에이전트를 골랐고 그 목록을 불러오는 중이 아닐 때만 true. FR-START-010 */
export function canReloadModels(state: SelectionState): boolean {
  const key = state.selectedKey;
  return key !== undefined && state.models[key]?.status !== 'loading';
}

/** 다음을 누를 때 넘길 값. 고르지 않았으면 undefined. */
export function toSelection(state: SelectionState): Selection | undefined {
  const choice = state.choices.find((candidate) => candidate.key === state.selectedKey);
  const list = choice === undefined ? undefined : readyList(state, choice.key);
  if (choice === undefined || list === undefined || state.model === undefined) return undefined;
  // 목록을 다시 불러오는 중이거나 고른 모델이 목록에 없으면 아직 넘길 수 없다
  if (!hasModel(list, state.model)) return undefined;
  return { agent: choice.agent, environment: choice.environment, model: state.model };
}
